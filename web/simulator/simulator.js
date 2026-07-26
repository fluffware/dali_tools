
function parse_rgb(s) {
    let m = s.match(/^rgb\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/i);
    if(!m) return null
    return [m[1],m[2],m[3]];
}

function send_variable_query(namess, vars) {
    let url = "/dyn/dali/device/?name="+namess.join(',')+ "&get="+vars.join(",");
    //console.log(url)
    fetch(url)
	.then(response => {
	    if (response.status == 200) {
		response.json().then(data => {
		    //console.log(data)
		    for ([name,param] of Object.entries(data)) {
			if (name == current_name_elem.value) {
			    for ([var_name, var_value] of Object.entries(param)) {
				let elems = gear_values[var_name]
			    for (elem of elems) {
				elem.textContent = var_value 
			    }
			    }
			}
			let filled = gear_filled[name]
			if (filled) {
			    for (fill of filled) {
				rgb = parse_rgb(fill.color)
				for (let i =0; i < 3; i++) {
				    rgb[i] = (rgb[i] *  param["actualLevel"])/254;
				}
				//console.log(`rgb(${rgb[0]},${rgb[1]},${rgb[2]})`)
				fill.elem.style.fill=`rgb(${rgb[0]},${rgb[1]},${rgb[2]})`;
			    }
			}
		    }
		})
	    } else {
		console.log("GET request failed: Status "+response.status)
	    }
	})
	.catch(err => {
	    console.log("Query failed: "+err)
	})
}

function send_variable_set(name, variable, value) {
    let url = "/dyn/dali/device/?name="+name+ "&set="+variable+":"+value
    //console.log(url)
    fetch(url)
	.then(response => {
	    if (response.status == 200) {
		response.json().then(data => {
		    // Request successful
		})
	    } else {
		console.log("GET request failed: Status "+response.status)
	    }
	})
	.catch(err => {
	    console.log("set failed: "+err)
	})
}

function dali_current_set(variable, value) {
    send_variable_set(current_name_elem.value, variable, value)
}

let current_name_elem = null;
function poll_vars() {
    let name = current_name_elem.value
    send_variable_query([name], Object.keys(gear_values))
}

function poll_intensity() {
    let names = Object.keys(gear_filled);
    if (names.length > 0) {
	send_variable_query(names, ["actualLevel"])
    }
}

function iframeRef( frameRef ) {
    return frameRef.contentWindow
        ? frameRef.contentWindow.document
        : frameRef.contentDocument
}

// Add to a map of arrays
function push_map(map, index, value) {
    let entry = map[index]
    if (entry == undefined) {
	map[index] = [value]
    } else {
	entry.push(value)
    }
}

function svg_elem_filled_clicked() {
    current_name_elem.value = this.getAttribute("data-dali-fill-intensity");
}

var gear_filled = {}

function layout_file_changed() {
    const file = this.files[0];
    console.log("File: "+file.name+" is "+file.type);
    let frame = document.getElementById("layout_frame")
    frame.src = URL.createObjectURL(file)
    setTimeout(function() {
	let layoyt_doc = iframeRef(frame)
	let svg = layoyt_doc.getElementsByTagName("svg")[0];
	let filled = svg.querySelectorAll("[data-dali-fill-intensity]")
	for (f of filled) {
	    let name = f.getAttribute("data-dali-fill-intensity");
	    console.log(f.style.fill)
	    push_map(gear_filled, name, {elem: f, color: f.style.fill})
	    f.addEventListener("click", svg_elem_filled_clicked);
	}
	
    }, 1000)
}
var gear_values = {};
function startup()
{
    for (elem of document.querySelectorAll("[data-dali-gear-value]")) {
	let var_name = elem.getAttribute("data-dali-gear-value")
	let entry = gear_values[var_name]
	if (entry == undefined) {
	    gear_values[var_name] = [elem]
	} else {
	    entry.push(elem)
	}
	console.log(var_name)
    }
    current_name_elem = document.getElementById("current_name")
    let layout_file_elem = document.getElementById("layout_file");
    layout_file_elem.addEventListener("change", layout_file_changed);

    console.log(gear_values)
    setInterval(poll_vars, 500);
    setInterval(poll_intensity, 100);
}
