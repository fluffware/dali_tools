let step_size = 10;
let addr_size = 10;
const  HYSTERESIS = 0.8;
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

let gear_index = 0;
let max_gear_index = 0;

const MASK=255;

const NO_COMMAND = 0;
const SCAN_INDEX = 1;
const FIND_ALL = 2;
const NEW_CONFIGURATION = 3;
const COMMIT_CHANGES = 4;
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
		    if (data.cmd != 0 && data.status=="Done") request_status()
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
	if (gear_index < max_gear_index) {
	    gear_index++;
	}
    } else {
	if (gear_index > 0) {
	    gear_index--;
	}
    }
    send_cmd(SCAN_INDEX, {index: gear_index})
    index_elem.innerText = (gear_index+1).toString();
}

function handle_reply(reply)
{
    gear_id = reply.gear_id;
    gear_label = reply.gear_id_label;
    max_gear_index = reply.length - 1;
    gear_index = reply.index;
    index_elem.innerText = gear_index + 1
    address_elem.innerText = gear_label;
    max_index_elem.innerText = max_gear_index + 1;
    new_address_elem.innerText = reply.new_conf_label;

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
    
}






class SwipeListener
{
    constructor(swipe) {
	this.swipe = swipe
	this.swiping = false
	let listener = this;
	swipe.addEventListener("click", function (event) {
	    event.preventDefault();
	})
	this.move = ev => listener.#swipe_listener(ev)
	swipe.addEventListener("mousedown", function (event) {
	    event.preventDefault();
	    if (!listener.swiping) {
		listener.swipe.addEventListener("mousemove", listener.move);
		listener.start_swipe(event.offsetX, event.offsetY);
		listener.swiping = true
	    }
	})

	swipe.addEventListener("touchstart", function (event) {
	    event.preventDefault();
	    if (!listener.swiping) {
		listener.swipe.addEventListener("touchmove", listener.move);
		let bbox = listener.swipe.getBoundingClientRect();
		listener.start_swipe(event.touches[0].clientX-bbox.left,event.touches[0].clientY - bbox.top);
		listener.swiping = true
	    }
	})

	swipe.addEventListener("mouseup", function (event) {
	    event.preventDefault();
	    listener.#_stop_swipe();

	})

	swipe.addEventListener("touchend", function (event) {
	    event.preventDefault();
	    listener.#_stop_swipe();
	})

	swipe.addEventListener("mouseout", function (event) {
	    event.preventDefault();
	    listener.#_stop_swipe();
	})

    }

    start_swipe(x,y) {
	console.log(`Start swipe: ${x}, ${y}`)
    }

    #_stop_swipe() {
	if (this.swiping) {
    	    this.swipe.removeEventListener("mousemove", this.move);
	    this.swipe.removeEventListener("touchmove", this.move);
	    this.swiping = false;
	}
	this.stop_swipe()
    }
    
    stop_swipe() {
	console.log(`Stop swipe`)
    }
    #swipe_listener(event)
    {
	let x,y;
	if (event.touches) {
	    let bbox = this.swipe.getBoundingClientRect();
	    x = event.touches[0].clientX - bbox.left;
	    y = event.touches[0].clientY - bbox.top;
	} else {
	    x = event.offsetX;
	    y = event.offsetY;
	}
	this.move_swipe(x,y)
    }
    
    move_swipe(x,y) {
	console.log(`Move swipe: ${x}, ${y}`)
    }
    
}

class IndexSwipeListener extends SwipeListener {
    constructor(swipe) {
	super(swipe);
	this.swipe_mode = SWIPE_NONE;
	this.swipe = swipe;
	this.step_x = 0;
	this.step_y = 0;
	this.ctxt = swipe.getContext("2d");
	this.interval = null;
	this.double_tap_timer = null;
    }

    step_size() {
	return (this.swipe.width + this.swipe.height) /30.0;
    }

    start_swipe(x,y)
    {
	this.swipe_mode = SWIPE_START;
	let step_size = this.step_size()
	this.step_x = x - step_size / 2;
	this.step_y = y - step_size / 2;
	console.log(step_size)
	this.ctxt.strokeStyle = "black"
	this.ctxt.lineWidth = 2
	this.ctxt.strokeRect(this.step_x, this.step_y, step_size,step_size);
	console.log(this.step_x,x)
	tps_current = 0;

    }
    
    stop_swipe()
    {
	if (this.swipe_mode != SWIPE_NONE) {
	    if (this.interval != null) {
		clearInterval(this.interval);
		this.interval = null;
	}
	    if (this.swipe_mode == SWIPE_START) {
		if (this.double_tap_timer) {
		    clearTimeout(this.double_tap_timer)
		    this.double_tap_timer = null
		    do_set_address()
		} else {
		    let listener = this
		    this.double_tap_timer = setTimeout(function() {
			listener.double_tap_timer = null
		    }, 300)
		}
	    }
	    this.swipe_mode = SWIPE_NONE;
	    this.ctxt.clearRect(0,0,swipe.width,swipe.height);
	}
    }

    #start_tick(tps)
    {
	if (this.interval != null) {
	    clearInterval(this.interval);
	}
	if (tps != 0) {
	this.interval = setInterval(function()
				    {
				   step(tps);
			       },
			       1000/Math.abs(tps))
	}
    }

    move_swipe(x,y) {
	let ctx = this.ctxt;
	let step_size = this.step_size()
	if (this.swipe_mode == SWIPE_START) {
	    if (y < this.step_y || y >= this.step_y + step_size) {
		this.swipe_mode = SWIPE_VERT;
	    } else if (x < this.step_x || x >= this.step_x + step_size) {
		this.swipe_mode = SWIPE_HORIZ;
	    }
	}
	if (this.swipe_mode == SWIPE_VERT) {
	    if (y < this.step_y) {
		this.step_y -= step_size * HYSTERESIS;
		step(1)
	    } else if (y >= this.step_y + step_size) {
		this.step_y += step_size * HYSTERESIS;
		step(-1)
	    }
	    ctx.clearRect(0,0,this.swipe.width,this.swipe.height);
	    ctx.beginPath();
	    ctx.moveTo(0,this.step_y);
	    ctx.lineTo(this.swipe.width, this.step_y);
	    ctx.moveTo(0,this.step_y+step_size);
	    ctx.lineTo(this.swipe.width, this.step_y+step_size);
	    ctx.stroke();
	    ctx.textAlign = "left"
	    ctx.textBaseline = "middle"
	    ctx.font = step_size/2 + "px sans"
	    if (gear_index < max_gear_index) {
		ctx.fillText(gear_index + 2, 10,this.step_y - 0.5 * step_size)
	    }
	    ctx.fillText(gear_index +1, 10,this.step_y + 0.5 * step_size)
	    if (gear_index > 0) {
		ctx.fillText(gear_index, 10,this.step_y + 1.5 * step_size)
	    }
	} else if (this.swipe_mode == SWIPE_HORIZ) {
	    if (x < this.step_x) {
		console.log(`Step x: ${this.step_x} ${step_size}`)
		this.step_x -= step_size * HYSTERESIS;
		console.log(`Step x: ${this.step_x} ${x}`)
		if (tps_current > -10) {
		    tps_current -= 1
		    this.#start_tick(tps_current)
		}
	    } else if (x >= this.step_x + step_size) {
		this.step_x += step_size * HYSTERESIS;
		if (tps_current < 10) {
		    tps_current += 1
		    this.#start_tick(tps_current)
		}
	    }
	    ctx.clearRect(0,0,this.swipe.width,this.swipe.height);
	    ctx.beginPath();
	    ctx.moveTo(this.step_x, 0);
	    ctx.lineTo(this.step_x, this.swipe.height);
	    ctx.moveTo(this.step_x+step_size, 0);
	    ctx.lineTo(this.step_x+step_size, this.swipe.height);
	    ctx.stroke();
	    ctx.textAlign = "center"
	    ctx.textBaseline = "top"
	    ctx.font = step_size/2 + "px sans"
	    if (tps_current > -10) {
		ctx.fillText(tps_current - 1, this.step_x - 0.5 * step_size, 10)
	    }
	    ctx.fillText(tps_current, this.step_x + 0.5 * step_size, 10)
	    if (tps_current < 10) {
		ctx.fillText(tps_current + 1, this.step_x + 1.5 * step_size, 10)
	    }
	}
    }
    
    
}

class AddressSwipeListener extends SwipeListener {
    constructor(swipe) {
	super(swipe);
	this.swipe = swipe;
	this.step_y = 0;
	this.ctxt = swipe.getContext("2d");
    }

    step_size() {
	return this.swipe.height/20.0;
    }

    start_swipe(x,y)
    {
	let step_size = this.step_size()
	this.step_y = y - step_size / 2;
	console.log(step_size)
	this.move_swipe(x,y)

    }
    
    stop_swipe()
    {
	this.ctxt.clearRect(0,0,swipe.width,swipe.height);
    }

   
    move_swipe(x,y) {
	console.log(`Move: ${y}`)
	if (conf_info.length == 0) return
	let ctx = this.ctxt;
	let step_size = this.step_size()
	let index =conf_entry_elem.selectedIndex;
	if (y < this.step_y) {
	    this.step_y -= step_size * HYSTERESIS;
	    if (index < conf_info.length - 1) {
		index++
	    }
	} else if (y >= this.step_y + step_size) {
	    this.step_y += step_size * HYSTERESIS;
	    if (index > 0) {
		index--
	    }
	}
	conf_entry_elem.selectedIndex = index
	ctx.clearRect(0,0,this.swipe.width,this.swipe.height);
	ctx.beginPath();
	ctx.moveTo(0,this.step_y);
	ctx.lineTo(this.swipe.width, this.step_y);
	ctx.moveTo(0,this.step_y+step_size);
	ctx.lineTo(this.swipe.width, this.step_y+step_size);
	ctx.stroke();
	ctx.textAlign = "left"
	ctx.textBaseline = "middle"
	ctx.font = step_size/2 + "px sans"
	index = conf_entry_elem.selectedIndex;
	console.log(`Index: ${index}`)
	let next_conf = conf_info[index + 1];
	if (next_conf != null) {
	    ctx.fillText(next_conf.conf_label, 10,this.step_y - 0.5 * step_size)
	}
	
	ctx.fillText(conf_info[index].conf_label, 10,this.step_y + 0.5 * step_size)
	let prev_conf = conf_info[index - 1];
	if (prev_conf != null) {
	    ctx.fillText(prev_conf.conf_label, 10,this.step_y + 1.5 * step_size)
	}
    }
    
    
}

function do_set_address() {
    let conf_index = conf_entry_elem.selectedIndex;
    if (conf_index!=null) {
	let conf_id = conf_info[conf_index]
	send_cmd(NEW_CONF, {id: conf_index, index: gear_index})
	conf_index++;
	if (conf_index >= conf_entry_elem.length) {
	    conf_entry_elem.selectedIndex = 0;
	} else {
	    conf_entry_elem.selectedIndex = conf_index + 1;
	}
    }
}

function resize_canvas(canvas)
{
    let part = canvas.parentElement;
    let resize = new ResizeObserver((entries) => {
	canvas.style.width = '100%';
	canvas.style.height = '100%';
	canvas.width = canvas.offsetWidth;
	canvas.height = canvas.offsetHeight;
	step_size = (canvas.width + canvas.height) /10.0  
	
    });
    resize.observe(part);
}

let conf_entry_elem = null
let conf_info = []

function get_configuration(conf_index) {
    let url = "/dyn/configuration?"+conf_index
    fetch(url)
	.then(response => {
	    if (response.status == 200) {
		response.json().then(data => {
		    console.log(data)
		    let opt_elem = document.createElement("option")
		    opt_elem.innerText = data.conf_label
		    opt_elem.setAttribute("value", conf_index);
		    conf_info[conf_index] = data
		    conf_entry_elem.append(opt_elem)			   
		    conf_index++
		    if (data.length > conf_index) {
			setTimeout(() => get_configuration(conf_index), 100);
			
		    }
		})
	    } else {
		console.log("GET request failed: Status "+response.status)
	    }
	})
}

function startup()
{
    body = document.body;
    tick = document.getElementById("tick");
    swipe = document.getElementById("swipe");
    addr_swipe = document.getElementById("addr_swipe");
    address_elem = document.getElementById("address");
    new_address_elem = document.getElementById("new_address");
    index_elem = document.getElementById("index");
    max_index_elem = document.getElementById("max_index");
    conf_entry_elem = document.getElementById("conf_entry");
    wait_elem = document.getElementById("wait");
    
    resize_canvas(swipe)
    resize_canvas(addr_swipe)
    

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

    index_listener = new IndexSwipeListener(swipe)
    addr_listener = new AddressSwipeListener(addr_swipe)
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
	    {
		e.preventDefault();
		let index =  conf_entry_elem.selectedIndex
		if (index < conf_entry_elem.length - 1) {
		    conf_entry_elem.selectedIndex = index + 1
		}
	    }
	    break
	case "ArrowDown":
	    {
		e.preventDefault();
		let index =  conf_entry_elem.selectedIndex
		if (index > 0) {
		    conf_entry_elem.selectedIndex = index - 1
		}
	    }
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

    conf_entry_elem.replaceChildren()
    conf_info = []
    get_configuration(0)
  
   
}
