
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
	if (draw_height * video_settings.width >= draw_width * video_settings.height) {
	    draw_height = Math.round(draw_width * video_settings.height / video_settings.width);
	} else {
	    draw_width = Math.round(draw_height * video_settings.width / video_settings.height);
	}
	capture.height = draw_height;
	capture.width = draw_width;
	captureCtxt.fillRect(0,0,draw_width, draw_height);

	var sense_x = Math.floor(draw_width / 4);
	var sense_y = Math.floor(draw_height / 4);
	var sense_width = Math.floor(draw_width / 2);
	var sense_height = Math.floor(draw_height / 2);
	var trend_pos= 0;
	var prev_intensity;
	var diff_filter = [0,0,0];
	var max_peak = 0;
	var prev_peak = undefined;
	var pos = 0;
	setInterval(
	    function() {
		captureCtxt.drawImage(video, 0,0,draw_width,draw_height);
		var pixels =
		    captureCtxt.getImageData(0, 0,
					     draw_width,draw_height).data;
		var full_sum = 0;
		for (p = 0; p < draw_height * draw_width*4; p+= 4) {
		    full_sum += pixels[p] + pixels[p+1] + pixels[p+2]; 
		}
		var sense_sum = 0;
		for (y = sense_y; y < sense_y + sense_height; y++) {
		    let p_start = (y*draw_width+sense_x) * 4;
		    let p_end = p_start + sense_width * 4;
		    for (p = p_start; p < p_end; p+= 4) {
			sense_sum += pixels[p] + pixels[p+1] + pixels[p+2];
		    }
		}
		var sense_intensity = sense_sum/(sense_width*sense_height*3);
		var full_intensity = full_sum/(draw_width*draw_height*3);
		//console.log(intensity);
		trendCtxt.fillStyle = "#000";
		trendCtxt.fillRect(trend_pos, 0, 1, trend.height);
		trendCtxt.fillStyle = "#fff";
		trendCtxt.fillRect(trend_pos, 50, 1, 1);
		
		trendCtxt.fillStyle = "#f00";
		trendCtxt.fillRect(trend_pos, sense_intensity * 50 / full_intensity, 1, 1);
		if (trend_pos >= trend.width) trend_pos = 0;
		

		let diff = sense_intensity - prev_intensity;
		prev_intensity = sense_intensity;

		let filtered = diff;
		for (let i = 0; i < diff_filter.length - 1; i++) {
		    filtered += diff_filter[i];
		    diff_filter[i] = diff_filter[i + 1];
		}
		filtered += diff_filter[diff_filter.length - 1];
		diff_filter[diff_filter.length - 1] = diff;
		diff = filtered / (diff_filter.length + 1);

		trendCtxt.fillStyle = "#0f0";
		trendCtxt.fillRect(trend_pos, diff * 5 + 128, 1, 1);
		trend_pos += 1;

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

