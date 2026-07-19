use clap::{Arg, Command};
use dali::drivers::driver::{DaliDriver, DaliFrame, OpenError};
use dali::drivers::send_flags;
use dali::httpd::{self, ServerConfig};
use dali::simulator;
use dali::simulator::device::DaliSimDevice;
use dali::simulator::timing;
use dali_tools as dali;
use dali_tools::simulator::sim_bus::{DaliSimBusDevice, DaliSimBusDeviceEvent};
use futures::FutureExt;
use futures::future::{Fuse, FusedFuture};
use hyper::{Body, Request, Response};
use hyper::{header, http};
use log::debug;
use log::error;
use std::collections::BTreeMap;
use std::fs::File;
use std::str::FromStr;
use std::time::Duration;
use tokio::signal;
use tokio_util::sync::CancellationToken;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn bad_request(msg: &str) -> DynResult<Response<Body>> {
    Response::builder()
        .status(http::StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(msg.to_owned()))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

fn decode_get_request(
    req: Request<Body>,
    sim_devices: &Vec<Box<dyn DaliSimDevice>>,
) -> DynResult<Response<Body>> {
    if let Some(addr) = req.uri().path().strip_prefix("/dyn/dali/device/") {
        let Ok(addr) = u8::from_str(addr) else {
            return bad_request("Invalid device address");
        };
        let mut sim_device = None;
        for dev in sim_devices {
            if let Ok(Ok(dev_addr)) = dev.get_parameter("shortAddress").map(|v| u8::from_str(&v)) {
                debug!("Checking dev {}", dev_addr);
                if dev_addr == addr {
                    sim_device = Some(dev);
                    break;
                }
            }
        }
        let Some(sim_device) = sim_device else {
            return bad_request("No device with given address");
        };
        let mut values = BTreeMap::new();
        if let Some(query) = req.uri().query() {
            let mut query_parts = query.split('&');
            for kv in query_parts {
                let Some((k, p)) = kv.split_once('=') else {
                    return bad_request("Missing '='");
                };
                match k {
                    "get" => {
                        let Ok(v) = sim_device.get_parameter(p) else {
                            return bad_request(&format!("No parameter named '{}' found", p));
                        };
                        values.insert(p, v);
                    }
                    _ => return bad_request(&format!("'{}' not supported", k)),
                }
            }
        }
        let reply = "{".to_string()
            + &values
                .iter()
                .map(|(k, v)| "\"".to_string() + k + "\":" + v)
                .collect::<Vec<String>>()
                .join(",")
            + "}";
        Response::builder()
            .status(http::StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(reply))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    } else {
        Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("No such command".to_string()))
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
			    tokio::time::sleep_until((start - delay).into()).await;
			    driver.send_frame(frame,send_flags::Flags::Priority(1)).await;
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
                .default_value("default")
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

    let mut web_server = {
        let mut web_conf = ServerConfig::new();
        web_conf = web_conf.port(1122);
        web_conf = web_conf.build_page(Box::new(move |req| decode_get_request(req, &sim_devices)));
        match httpd::start(web_conf) {
            Ok((server, _bound_ip, _bound_port)) => server.fuse(),
            Err(e) => {
                eprintln!("Failed to start web server");
                return;
            }
        }
    };
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);
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
    if !dali_hw.is_terminated() {
        let _ = dali_hw.await;
    }
    debug!("Exiting");
}
