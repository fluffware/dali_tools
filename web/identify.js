
function startup()
{
    console.log("Started");
    
    var video_settings;
    
    var capture = document.getElementById("capture");
    var captureCtxt = capture.getContext("2d");
    var trend = document.getElementById("trend");
    var trendCtxt = trend.getContext("2d");
    var progress = document.getElementById("progress");
    var address_elem = document.getElementById("address");
    trendCtxt.fillRect(0,0,trend.width, trend.height);

    var video = document.getElementById("live")
    if (!video) {
	alert("No video element found");
	return;
    } 
    if (navigator.mediaDevices && navigator.mediaDevices.getUserMedia) {
	navigator.mediaDevices.getUserMedia({video: true}).then(
	    function(stream) {
		if (!stream) {
		    alert("No camera stream found");
		    return;
		}
		console.log(stream);
		video.srcObject = stream;
		video.play();
		var tracks = stream.getVideoTracks();
		if (tracks.length == 0) {
		    alert("No video tracks");
		    return;
		}
		video_settings = tracks[0].getSettings();
		
		start_capture();
	    });
    } else {
	alert("Camera not supported");
    }

    function start_capture()
    {
	var draw_width = 50;
	var draw_height = 50;
	console.log(video_settings);
	var aspect = video_settings.aspectRatio;
	if (draw_height * aspect >= draw_width) {
	    draw_height = draw_width / aspect;
	} else {
	    draw_width = draw_height * aspect;
	}
	capture.height = draw_height;
	capture.width = draw_width;
	captureCtxt.fillRect(0,0,draw_width, draw_height);
	
	var data_width = 13;
	var data_height = 13;
	var trend_pos= 0;
	var prev= [0,0];
	var max_peak = 0;
	var prev_peak = undefined;
	var pos = 0;
	setInterval(
	    function() {
		captureCtxt.drawImage(video, 0,0,draw_width,draw_height);
		var pixels =
		    captureCtxt.getImageData((draw_width-data_width)/2,
					     (draw_height-data_height)/2,
					     data_width,data_height).data;
		var sum = 0;
		for (p = 0; p < data_width * data_width*4; p+= 4) {
		    sum += pixels[p] + pixels[p+1] + pixels[p+2]; 
		}
		var intensity = sum/(data_width*data_height*3);
		//console.log(intensity);
		trendCtxt.fillStyle = "#000";
		trendCtxt.fillRect(trend_pos, 0, 1, trend.height);
		trendCtxt.fillStyle = "#fff";
		trendCtxt.fillRect(trend_pos, (intensity - prev[1])*5 + 128, 1, 1);
		trend_pos += 1;
		if (trend_pos >= trend.width) trend_pos = 0;
		
		prev[1] = prev[0];
		prev[0] = intensity;
		let diff = intensity - prev[1];
		if (max_peak < Math.abs(diff)) {
		    max_peak = Math.abs(diff);
		}
		
		let peak_limit = max_peak * 0.5;

		if (Math.abs(diff) > peak_limit) {
		    if (prev_peak) {
			// Replace this peak with the previous if it's bigger
			// otherwise ignore
			if ((diff > 0 && prev_peak.value < diff) 
			    || (diff < 0 && prev_peak.value > diff)) {
			    prev_peak.pos++;
			    prev_peak.value = diff;
			    pos = 0;
			} else {
			    decode(prev_peak);
			    prev_peak = undefined;
			}
		    } else {
			prev_peak = {value: diff, pos: pos}
			pos = 0;
		    }
		} else {
		    if (prev_peak) decode(prev_peak);
		    prev_peak = undefined;
		}
		pos++;
		max_peak *= .99;
	    },
	    50);
    }

    var last_peak_value = 0;
    var half_bits = 0;
    var prev_bits = 0;
    var bit_value = 0;
    function decode(peak) {
	let pos = peak.pos
	if (half_bits == 0) {
	    if (pos <= 11 && pos >= 9) {
		half_bits += 2;
		bit_value = 0;
	    }
	} else {
	    
	    if (last_peak_value * peak.value > 0) {
		// Peaks must have alternating signs
		half_bits = 0;
		prev_bits = 2;
	    } else {
		if (pos >= 4 && pos <= 6 &&  (half_bits & 1) == 0) {
		    half_bits += 1;
		    prev_bits = 1;
		} else if (pos >= 9 && pos <= 11) {
		    half_bits += 2;
		    prev_bits = 2;
		} else if (pos >= 14 && pos <= 16
			   && (half_bits & 1) == 1) {
		    half_bits += 3;
		    prev_bits = 3;
		} else {
		    half_bits = 0;
		}
		bit_value = bit_value>>1 | ((half_bits & 1) * 32);
		console.log(bit_value)
	    }
	}
	last_peak_value = peak.value;
	console.log(peak);
	console.log(half_bits);
	if (half_bits == 13) {
	    half_bits = 14;
	}
	if ( address_elem) {
	    if (half_bits == 14) {
		address_elem.innerText = bit_value;
	    } else if (half_bits != 0) {
		address_elem.innerText ="";
	    }
	}
	if (progress) {
	    let percent = half_bits * 100 / 14;
	    progress.innerText = `${percent.toFixed(0)}`;
	}
    }

}

