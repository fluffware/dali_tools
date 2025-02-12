let step_x = 0;
let step_y = 0;
let step_size = 10;
let hysteresis = 10;
const SWIPE_NONE = 0;
const SWIPE_START = 1;
const SWIPE_HORIZ = 2;
const SWIPE_VERT = 3;
let swipe_mode = SWIPE_NONE;

let tick;
let interval = null;
let tps_current = 0;
address_elem = null;
index_elem = null;
max_index_elem = null;
wait_elem = null;

let address = 0;
let index = 0;
let max_index = 0;

const MASK=255;

const NO_COMMAND = 0;
const SCAN_ADDRESS = 1;
const FIND_ALL = 2;
const NEW_ADDRESS = 3;
const CHANGE_ADDRESSES = 4;
const SORT = 5;

let executing = {}
function send_cmd(cmd, args = {}) {
    let entries = Object.entries(args);
    let url = "/dyn/dali?cmd="+cmd
    if (entries.length > 0) {
	for (const [key, value] of entries) {
	    url += `&${key}=${value}`
	}
    }	    
    fetch(url)
	.then(response => {
	    if (response.status == 202) {
		response.json().then(data => {
		    console.log(data)
		    executing[data.id] = data
		})
	    } else {
		console.log("GET request failed: Status "+response.status)
	    }
	})
	.catch(err => {
	    console.log("Command "+cmd+" failed: "+err)
	})
}

function request_status() {
    fetch("/dyn/cmd_status")
	.then(response => {
	    if (response.status == 200) {
		response.json().then(data => {
		    if (wait_elem) {
			wait_elem.style.visibility = data.cmd != 0 ? "visible" : "hidden";
		    }
		    if (data.cmd != 0) request_status()
		    console.log(data)
		})
	    } else {
		console.log("Status request failed: Status "+response.status)
	    }
	})
}
function request_scan_state(cmd, args = {}) {
    fetch("/dyn/scan_state")
	.then(response => {
	    if (response.status == 200) {
		response.json().then(data => {
		    console.log(data)
		    handle_reply(data)
		})
	    } else {
		console.log("Scan state request failed: Status "+response.status)
	    }
	})
}

function step(dir) {
//    tick.play();
    if (dir > 0) {
	if (index < max_index) {
	    index++;
	}
    } else {
	if (index > 0) {
	    index--;
	}
    }
    send_cmd(SCAN_ADDRESS, {index: index})
    index_elem.innerText = (index+1).toString();
}

function handle_reply(reply)
{
    address = reply.current_address;
    max_index = reply.length - 1;
    index = reply.index;
    index_elem.innerText = index + 1
    address_elem.innerText = address + 1;
    max_index_elem.innerText = max_index + 1;
    new_address_elem.innerText = 
	reply.new_address == MASK 
	? "-":(reply.new_address+1);
}

function start_tick(tps)
{
    if (interval != null) {
	clearInterval(interval);
    }
    if (tps != 0) {
	interval = setInterval(function()
			       {
				   step(tps);
			       },
			       1000/Math.abs(tps))
    }
}

function swipe_listener(event)
{
    let x,y;
    if (event.touches) {
	let bbox = swipe.getBoundingClientRect();
	x = event.touches[0].clientX - bbox.left;
	y = event.touches[0].clientY - bbox.top;
    } else {
	x = event.offsetX;
	y = event.offsetY;
    }
    //console.log("swipe x", x , step_x);
    //console.log("swipe y", y , step_y);
    let ctx = swipe.getContext("2d");
    if (swipe_mode == SWIPE_START) {
	if (y < step_y || y >= step_y + step_size) {
	    swipe_mode = SWIPE_VERT;
	} else if (x < step_x || x >= step_x + step_size) {
	    swipe_mode = SWIPE_HORIZ;
	}
    }
    if (swipe_mode == SWIPE_VERT) {
	if (y < step_y) {
	    step_y -= step_size - hysteresis;
	    step(1)
	} else if (y >= step_y + step_size) {
	    step_y += step_size-hysteresis;
	    step(-1)
	}
	ctx.clearRect(0,0,swipe.width,swipe.height);
	ctx.beginPath();
	ctx.moveTo(0,step_y);
	ctx.lineTo(swipe.width, step_y);
	ctx.moveTo(0,step_y+step_size);
	ctx.lineTo(swipe.width, step_y+step_size);
	ctx.stroke();
	ctx.textAlign = "left"
	ctx.textBaseline = "middle"
	ctx.font = step_size/2 + "px sans"
	if (index < max_index) {
	    ctx.fillText(index + 2, 10,step_y - 0.5 * step_size)
	}
	ctx.fillText(index +1, 10,step_y + 0.5 * step_size)
	if (index > 0) {
	    ctx.fillText(index, 10,step_y + 1.5 * step_size)
	}
    } else if (swipe_mode == SWIPE_HORIZ) {
	if (x < step_x) {
	    step_x -= step_size - hysteresis;
	    if (tps_current > -10) {
		tps_current -= 1
		start_tick(tps_current)
	    }
	} else if (x >= step_x + step_size) {
	    step_x += step_size-hysteresis;
	    if (tps_current < 10) {
		tps_current += 1
		start_tick(tps_current)
	    }
	}
	ctx.clearRect(0,0,swipe.width,swipe.height);
	ctx.beginPath();
	ctx.moveTo(step_x, 0);
	ctx.lineTo(step_x, swipe.height);
	ctx.moveTo(step_x+step_size, 0);
	ctx.lineTo(step_x+step_size, swipe.height);
	ctx.stroke();
	ctx.textAlign = "center"
	ctx.textBaseline = "top"
	ctx.font = step_size/2 + "px sans"
	if (tps_current > -10) {
	    ctx.fillText(tps_current - 1, step_x - 0.5 * step_size, 10)
	}
	ctx.fillText(tps_current, step_x + 0.5 * step_size, 10)
	if (tps_current < 10) {
	    ctx.fillText(tps_current + 1, step_x + 1.5 * step_size, 10)
	}
    }
}

let swipe;

let double_tap_timer = null;

function stop_swipe()
{
    if (swipe_mode != SWIPE_NONE) {
	swipe.removeEventListener("mousemove", swipe_listener);
	swipe.removeEventListener("touchmove", swipe_listener);
	if (interval != null) {
	     clearInterval(interval);
	    interval = null;
	}
	if (swipe_mode == SWIPE_START) {
	    if (double_tap_timer) {
		clearTimeout(double_tap_timer)
		double_tap_timer = null
		do_set_address()
	    } else {
		double_tap_timer = setTimeout(function() {
		    double_tap_timer = null
		}, 300)
	    }
	}
	swipe_mode = SWIPE_NONE;
	var ctx = swipe.getContext("2d");
	ctx.clearRect(0,0,swipe.width,swipe.height);
    }
}

function start_swipe(x,y)
{
    swipe_mode = SWIPE_START;
    step_x = x - step_size / 2;
    step_y = y - step_size / 2;
    var ctx = swipe.getContext("2d");
    console.log(step_size)
    ctx.strokeStyle = "black"
    ctx.lineWidth = 2
    ctx.strokeRect(step_x, step_y, step_size,step_size);
    swipe.width
    tps_current = 0;

}


function do_set_address() {
    let new_addr = parseInt(address_entry_elem.value) - 1;
    if (new_addr!=null) {
	send_cmd(NEW_ADDRESS, {address: new_addr, index: index})
	new_addr++;
	if (new_addr >= 64) {
	    address_entry_elem.value = 1;
	} else {
	    address_entry_elem.value = new_addr + 1;
	}
    }
}


function startup()
{
    body = document.body;
    tick = document.getElementById("tick");
    swipe = document.getElementById("swipe");
    address_elem = document.getElementById("address");
    new_address_elem = document.getElementById("new_address");
    index_elem = document.getElementById("index");
    max_index_elem = document.getElementById("max_index");
    address_entry_elem = document.getElementById("address_entry");
    wait_elem = document.getElementById("wait");
    swipe.addEventListener("click", function (event) {
	event.preventDefault();
    })
    swipe.addEventListener("mousedown", function (event) {
	event.preventDefault();
	console.log("mousedown");
	if (swipe_mode == SWIPE_NONE) {
	    swipe.addEventListener("mousemove", swipe_listener);
	    start_swipe(event.offsetX, event.offsetY);
	}
    })

    swipe.addEventListener("touchstart", function (event) {
	event.preventDefault();
	console.log("touchstart");
	if (swipe_mode == SWIPE_NONE) {
	    swipe.addEventListener("touchmove", swipe_listener);
	    let bbox = swipe.getBoundingClientRect();
	    start_swipe(event.touches[0].clientX-bbox.left,event.touches[0].clientY - bbox.top);
	}
    })

    swipe.addEventListener("mouseup", function (event) {
	event.preventDefault();
	console.log("mouseup");
	stop_swipe();

    })

    swipe.addEventListener("touchend", function (event) {
	event.preventDefault();
	console.log("mouseup");
	stop_swipe();
    })

    swipe.addEventListener("mouseout", function (event) {
	event.preventDefault();
	console.log("mouseout");
	stop_swipe();
    })
    
    let low_part = document.getElementById("low_box");
    let resize = new ResizeObserver((entries) => {
	swipe.style.width = '100%';
	swipe.style.height = '100%';
	swipe.width = swipe.offsetWidth;
	swipe.height = swipe.offsetHeight;
	step_size = (swipe.width + swipe.height) /30.0
	
    });
    resize.observe(low_part);

   

    let find_all = document.getElementById("find_all");
    find_all.addEventListener("click", function() {
	send_cmd(FIND_ALL)
    });

    let set_address = document.getElementById("set_address");
    set_address.addEventListener("click", function() {
	do_set_address()
    });

    let change_addresses = document.getElementById("change_addresses");
    change_addresses.addEventListener("click", function() {
	send_cmd(CHANGE_ADDRESSES)
    });
    
    let sort = document.getElementById("sort");
    sort.addEventListener("click", function() {
	send_cmd(SORT)
    });

    body.addEventListener("keydown", function(e) {
	switch(e.code) {
	case "ArrowLeft":
	    e.preventDefault();
	    step(-1)
	    break
	case "ArrowRight":
	    e.preventDefault();
	    step(1)
	    break
	case "ArrowUp":
	    e.preventDefault();
	    address_entry_elem.stepUp()
	    break
	case "ArrowDown":
	    e.preventDefault();
	    address_entry_elem.stepDown()
	    break
	case "Enter":
	case "Space":
	    e.preventDefault();
	    do_set_address();
	    break
	}
	console.log("Keydown"+e.code);
    })

    body.addEventListener("keyup", function(e) {
	console.log("Key up");
    })
    setInterval( function() {
	request_status()
	request_scan_state()
    }, 1000)
    send_cmd(FIND_ALL)
   
}
