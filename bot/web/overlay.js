/**
 * Get the current URL to connect to.
 */
function url() {
    var loc = window.location;
    var scheme = "ws";

    if (loc.protocol === "https:") {
        scheme = "wss";
    }

    let path = loc.pathname.split("/");
    path = path.slice(0, path.length - 1).join("/");

    return `${scheme}://${loc.host}${path}/ws`;
}

class Handlers {
    constructor() {
        this.handlers = {};
    }

    /**
     * Insert a handlers.
     * @param {string} key
     * @param {function} cb
     */
    insert(key, cb) {
        this.handlers[key] = cb;
    }

    /**
     * Call the handler associated with the given data.
     *
     * @param {object} data
     */
    call(data) {
        let cb = this.handlers[data.type];

        console.log("ws", data);

        if (cb !== undefined) {
            cb(data);
        }
    }
}

/**
 * Format duration in a human-readable way.
 * @param {*} duration
 */
function formatDuration(duration) {
    let seconds = duration % 60;
    let minutes = Math.floor(duration / 60);

    return pad(minutes, 2) + ":" + pad(seconds, 2);

    function pad(num, size) {
        var s = num + "";

        while (s.length < size) {
            s = "0" + s;
        }

        return s;
    }
}

class CurrentSong {
    constructor(elem) {
        this.name = elem.querySelector(".name");
        this.artist = elem.querySelector(".artist");
        this.artistName = elem.querySelector(".artist-name");
        this.state = elem.querySelector(".state");
        this.progress = elem.querySelector(".progress-bar");
        this.albumArt = elem.querySelector(".album-art");
        this.progress = elem.querySelector(".progress-bar");
    }

    /**
     * Update track information.
     * @param {object} data
     */
    update(data) {
        if (data.track === null) {
            this.name.textContent = "No Track";
            this.artistName = "";
            this.artist.style.display = "none";
            albumArt.style.display = "none";
        } else {
            this.name.textContent = data.track.name;
            this.artistName.textContent = data.track.artists[0].name;
            this.artist.style.display = "inline";
            this.albumArt.style.display = "inline-block";

            let image = this.pickAlbumArt(data.track.album.images, 64);

            if (image !== null) {
                this.albumArt.style.display = "inline-block";
                this.albumArt.src = image.url;
                this.albumArt.width = image.width;
                this.albumArt.height = image.height;
            }
        }

        if (data.paused) {
            state.innerHTML = "&#9208;";
        } else {
            state.innerHTML = "&#9654;";
        }
    }

    /**
     * Update the progress for the progress bar.
     * @param {object} data
     */
    updateProgress(data) {
        if (!this.progress) {
            return;
        }

        let p = Math.round((data.elapsed / data.duration) * 10000) / 100;
        this.progress.style.width = `${p}%`;
    }

    /**
     * Pick the image best suited for album art.
     */
    pickAlbumArt(images, smaller) {
        for (let i = 0; i < images.length; i++) {
            let image = images[i];

            if (image.width <= smaller && image.height <= smaller) {
                return image;
            }
        }

        return null;
    }
}

class Service {
    constructor() {
        this.handlers = new Handlers();
        let currentSong = document.getElementById("current-song");

        if (currentSong !== null) {
            this.currentSong = new CurrentSong(currentSong);

            this.handlers.insert("song/current", (data) => {
                this.currentSong.update(data);
            });

            this.handlers.insert("song/progress", (data) => {
                this.currentSong.updateProgress(data);
            });
        } else {
            this.currentSong = null;
        }
    }

    /**
     * Call the handler associated with the data received.
     *
     * @param {*} data
     */
    call(data) {
        this.handlers.call(data);
    }
}

function connect(service) {
    const ws = new WebSocket(url());

    ws.onmessage = (ev) => {
        var data;

        try {
            data = JSON.parse(ev.data);
        } catch(e) {
            console.log(`failed to parse message: ${ev.data}`);
            return;
        }

        service.call(data);
    };

    ws.onopen = (ev) => {
    };

    ws.onclose = function() {
        console.log("connection to server lost, reconnecting in 1s...");

        setTimeout(() => {
            connect(service);
        }, 1000);
    };
}

window.onload = () => {
    let service = new Service();
    connect(service);
}