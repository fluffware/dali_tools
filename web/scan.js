let ws;
let start_x = 0;
let start_y = 0;
let tick;
let interval = null;
let tps_current;
address_elem = null;
index_elem = null;
max_index_elem = null;

let address = 0;
let index = 0;
let max_index = 0;

const MASK=255;

function step(dir) {
    tick.play();
    if (dir > 0) {
	if (index < max_index) {
	    index++;
	}
    } else {
	if (index > 0) {
	    index--;
	}
    }
    ws.send(JSON.stringify({ ScanAddress: index }));
    index_elem.innerText = (index+1).toString();
}

function handle_reply(reply)
{
    if (reply.ScanUpdate) {
	address = reply.ScanUpdate.current_address;
	max_index = reply.ScanUpdate.length - 1;
	address_elem.innerText = address;
	max_index_elem.innerText = max_index + 1;
	new_address_elem.innerText = 
	    reply.ScanUpdate.new_address == MASK 
	    ? "-":reply.ScanUpdate.new_address;
    }
    console.log(reply);
}

function swipe_listener(event)
{
    let x,y;
    if (event.targetTouches) {
	x = event.targetTouches[0].clientX;
	y = event.targetTouches[0].clientY;
    } else {
	x = event.clientX;
	y = event.clientY;
    }
    console.log("swipe", x , start_x);

    let rel_pos = (x-start_x) / (swipe.getBoundingClientRect().width*0.5)
    let tps = Math.floor(Math.abs(rel_pos*rel_pos*12*2))
    if (tps > 20) tps = 20
    if (tps_current != tps) {
	if (interval != null) {
	    clearInterval(interval);
	}
	if (tps > 0) {
	    if (tps > tps_current) {
		step(x-start_x);
	    }
	    interval = setInterval(function()
				   {
				       step(x-start_x);
				   },
				   2000/tps)
	}
	tps_current = tps;
    }
    
    let diff_y = y - start_y;
    if (Math.abs(diff_y) > swipe.getBoundingClientRect().height*0.1) {
	step(start_y-y);
	start_y = y;
    }
}

let swipe;
let swiping = false;

function stop_swipe()
{
    if (swiping) {
	swipe.removeEventListener("mousemove", swipe_listener);
	swipe.removeEventListener("touchmove", swipe_listener);
	 if (interval != null) {
	     clearInterval(interval);
	     interval = null;
	 }
	swiping = false;
    }
}

function socket_uri() {
    var loc = window.location,
        new_uri;
    if (loc.protocol === "https:") {
        new_uri = "wss:";
    } else {
        new_uri = "ws:";
    }
    new_uri += "//" + loc.host;
    new_uri += "/socket/";
    return new_uri;
}

function startup()
{
    tick = document.getElementById("tick");
    swipe = document.getElementById("swipe");
    address_elem = document.getElementById("address");
    new_address_elem = document.getElementById("new_address");
    index_elem = document.getElementById("index");
    max_index_elem = document.getElementById("max_index");
    address_entry_elem = document.getElementById("address_entry");
    swipe.addEventListener("click", function (event) {
	event.preventDefault();
    })
    swipe.addEventListener("mousedown", function (event) {
	event.preventDefault();
	console.log("mousedown");
	if (!swiping) {
	    swipe.addEventListener("mousemove", swipe_listener);
	    swiping = true;
	    start_x = event.clientX;
	    start_y = event.clientY;
	}
    })

    swipe.addEventListener("touchstart", function (event) {
	event.preventDefault();
	console.log("touchstart");
	if (!swiping) {
	    swipe.addEventListener("touchmove", swipe_listener);
	    swiping = true;
	    start_x = event.targetTouches[0].clientX;
	    start_y = event.targetTouches[0].clientY;
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

    ws = new WebSocket(socket_uri());
    ws.onmessage = (msg) => {
        let reply = JSON.parse(msg.data);
	handle_reply(reply);
    }

    let find_all = document.getElementById("find_all");
    find_all.addEventListener("click", function() {
	ws.send(JSON.stringify({FindAll:true}))
    });

    let set_address = document.getElementById("set_address");
    set_address.addEventListener("click", function() {
	let new_addr = parseInt(address_entry_elem.value);
	if (new_addr!=null) { 
	    ws.send(JSON.stringify({NewAddress:{address: new_addr, index: index}}))
	    if (new_addr == 64) {
		address_entry_elem.value = 1;
	    } else {
		address_entry_elem.value = new_addr + 1;
	    }
	}
    });

    ws.onopen = (msg) => {
	ws.send(JSON.stringify({RequestScanUpdate:true}))
    }
}
