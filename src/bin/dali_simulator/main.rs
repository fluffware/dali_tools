use bytes::Bytes;
use clap::{Arg, Command};
use dali::drivers::driver::{DaliDriver, DaliFrame, OpenError};
use dali::drivers::send_flags;
use dali::httpd::{self, ServerConfig};
use dali::simulator;
use dali::simulator::device::{DaliSimDevice, ParameterError};
use dali::simulator::timing;
use dali_tools as dali;
use dali_tools::simulator::sim_bus::{DaliSimBusDevice, DaliSimBusDeviceEvent};
use futures::FutureExt;
use futures::future::{Fuse, FusedFuture};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper::{header, http};
use log::debug;
use log::error;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::str::FromStr;
use std::time::Duration;
use tokio::signal;
use tokio_util::sync::CancellationToken;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn bad_request(msg: &str) -> DynResult<Response<Full<Bytes>>> {
    Response::builder()
        .status(http::StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Full::new(Bytes::from(msg.to_owned())))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

fn decode_get_request(
    req: Request<Incoming>,
    sim_devices: &Vec<Box<dyn DaliSimDevice>>,
) -> DynResult<Response<Full<Bytes>>> {
    if let Some(_) = req.uri().path().strip_prefix("/dyn/dali/device") {
        let mut addrs = HashSet::new();
        let mut gets = HashSet::new();
        let mut sets = HashMap::new();

        if let Some(query) = req.uri().query() {
            let query_parts = query.split('&');
            for kv in query_parts {
                let Some((k, p)) = kv.split_once('=') else {
                    return bad_request("Missing '='");
                };
                match k {
                    "addr" => {
                        for s in p.split(",") {
                            let Ok(addr) = u8::from_str(s) else {
                                return bad_request("Invalid device address");
                            };
                            addrs.insert(addr);
                        }
                    }
                    "get" => {
                        for s in p.split(",") {
                            gets.insert(s);
                        }
                    }
                    "set" => {
                        for s in p.split(",") {
                            let Some((p, v)) = s.split_once(':') else {
                                return bad_request(
                                    "The argumet of set must be <parameter name>:<value>",
                                );
                            };
                            sets.insert(p, v);
                        }
                    }
                    _ => return bad_request(&format!("'{}' not supported", k)),
                }
            }
        }
        let mut reply = String::from("{");
        let mut first_addr = true;
        // Go through all devices and set and get parameters
        for dev in sim_devices {
            if let Ok(Ok(dev_addr)) = dev.get_parameter("shortAddress").map(|v| u8::from_str(&v)) {
                if addrs.contains(&dev_addr) {
                    for (p, v) in &sets {
                        match dev.set_parameter(p, v) {
                            Ok(()) => {}
                            Err(ParameterError::NotFound) => {
                                return bad_request(&format!("No parameter named '{}' found", p));
                            }
                            Err(ParameterError::InvalidValue) => {
                                return bad_request(&format!(
                                    "Invalid parameter value for '{}'",
                                    p
                                ));
                            }
                        }
                    }
                    if !first_addr {
                        reply += ",";
                    }
                    first_addr = false;
                    reply += &format!("\"{}\":{{", dev_addr);
                    let mut first_param = true;
                    for p in &gets {
                        let Ok(v) = dev.get_parameter(p) else {
                            return bad_request(&format!("No parameter named '{}' found", p));
                        };
                        if !first_param {
                            reply += ",";
                        }
                        first_param = false;
                        reply += &format!("\"{}\":{}", p, v);
                    }
                    reply += "}";
                }
            }
        }
        reply += "}";
        Response::builder()
            .status(http::StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(reply)))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    } else {
        Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Full::new(Bytes::from("No such command".to_string())))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}
#[cfg(feature = "sim_serial")]
mod sim_serial;
#[cfg(not(feature = "sim_serial"))]
mod sim_serial {
    use dali_tools::simulator::sim_bus::DaliSimBusDevice;
    use tokio_util::sync::CancellationToken;

    pub async fn start_serial(
        _bus_device: DaliSimBusDevice,
        _port_path: &str,
        cancel: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
        cancel.cancelled().await;
        Ok(())
    }
}

async fn dali_listener(
    mut driver: Box<dyn DaliDriver>,
    bus_device: DaliSimBusDevice,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        #[rustfmt::skip]
	tokio::select! {
	    res = driver.next_bus_event() =>  {
		match res {
		    Ok(event) => {
			if let Ok(frame) = DaliFrame::try_from(&event.event_type) {
			    let frame_end = event.timestamp - Duration::from_micros(2400);
			    let frame_start = frame_end - timing::frame_duration(&frame);
			    bus_device.add_event(event.event_type.clone(),
						 frame_end, Some(frame_start));
			    bus_device.wait_until(event.timestamp).await;
			}
			debug!("{:?}", event);
		    }
		    Err(e) => {
			return Err(format!("Reading event from DALI hardware failed: {}", e).into()); 
		    }
		}
	    }
	    res = bus_device.wait() => {
		match res {
		    DaliSimBusDeviceEvent::Shutdown => {
			return Ok(())
		    }
		    DaliSimBusDeviceEvent::Message(msg) => {
			if let Ok(frame) = DaliFrame::try_from(&msg.event_type)  && let Some(start) = msg.start {

			    let delay = match frame {
				DaliFrame::Frame8(_) => timing::REPLY_DELAY,
				DaliFrame::Frame16(_) |
				DaliFrame::Frame24(_) |
				DaliFrame::Frame25(_) => {
				    timing::send_delay(1,false)
				}
			    };
			    debug!("Delayed {:?}",start - delay);
			    tokio::time::sleep_until((start - delay).into()).await;
			    driver.send_frame(frame,send_flags::Flags::Priority(0)).await;
			}
		    }
		    DaliSimBusDeviceEvent::Timeout => {
			panic!("Timeout on wait");
		    }
		}
	    }
	}
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        eprintln!("Failed to initialize DALI drivers: {}", e);
    }
    let cli_cmd = Command::new("dali_simulator")
        .about("Simulate DALI-devices on a bus ")
        .arg(Arg::new("CONFIG").required(true).help("Configuration file"))
        .arg(
            Arg::new("SERIAL_DEVICE")
                .long("serial-device")
                .help("Serial device for simulation"),
        )
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .env("DALI_DEVICE")
                .help("Select DALI-device"),
        );
    let matches = cli_cmd.get_matches();
    let conf_filename = matches.get_one::<String>("CONFIG").unwrap();
    let conf_file = match File::open(conf_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "Failed to open configuration file '{}': {}",
                conf_filename, e
            );
            return;
        }
    };
    let (bus, mut sched, sim_devices) = match simulator::setup::setup_simulator(conf_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to start simulator: {}", e);
            return;
        }
    };
    let cancel = CancellationToken::new();
    let serial = if let Some(serial_path) = matches.get_one::<String>("SERIAL_DEVICE") {
        sim_serial::start_serial(
            DaliSimBusDevice::new(bus.clone(), sched.new_task()),
            &serial_path,
            cancel.clone(),
        )
        .fuse()
    } else {
        Fuse::terminated()
    };
    tokio::pin!(serial);

    let dali_hw = if let Some(device_name) = matches.get_one::<String>("DEVICE") {
        let driver = match dali::drivers::open(device_name) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to open DALI device '{}': {}", device_name, e);
                if let OpenError::NotFound = e {
                    eprintln!("Available drivers:");
                    for name in dali::drivers::driver_names() {
                        eprintln!("  {}", name);
                    }
                }
                return;
            }
        };

        dali_listener(driver, DaliSimBusDevice::new(bus.clone(), sched.new_task())).fuse()
    } else {
        Fuse::terminated()
    };
    tokio::pin!(dali_hw);

    let web_server = {
        let mut web_conf = ServerConfig::new();
        web_conf = web_conf.port(1122);
        web_conf = web_conf.build_page(Box::new(move |req| decode_get_request(req, &sim_devices)));
        match httpd::start(web_conf, cancel.cancelled()).await {
            Ok((server, _bound_ip, _bound_port)) => server.fuse(),
            Err(e) => {
                eprintln!("Failed to start web server: {e}");
                return;
            }
        }
    };
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);
    tokio::pin!(web_server);
    loop {
        #[rustfmt::skip]
        tokio::select! {
            res = &mut serial => {
		if let Err(e) = res {
                    error!("Serial pseudo port failed: {}",e);
		}
		break;
            }
	    res = &mut dali_hw => {
		if let Err(e) = res {
                    error!("DALI driver failed: {}",e);
		}
		break;
            }
            res = &mut ctrl_c => {
		if let Err(_) = res {
                    error!("Failed to wait for Ctrl-C");
		}
		break;
            }
	    res = &mut web_server => {
		if let Err(_) = res {
                    error!("Web server failed");
		}
		break;
	    }
        }
    }
    cancel.cancel();
    debug!("Cancelled");
    if !web_server.is_terminated() {
        let _ = web_server.await;
    }
    if !serial.is_terminated() {
        let _ = serial.await;
    }
    /*
    if !dali_hw.is_terminated() {
        let _ = dali_hw.await;
    }*/
    debug!("Exiting");
}
