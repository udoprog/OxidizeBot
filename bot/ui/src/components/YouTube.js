import {Spinner} from "../utils.js";
import React from "react";
import {Button, Table} from "react-bootstrap";
import {websocketUrl} from "../utils.js";
import Websocket from "react-websocket";

const OBS_CSS = [
  "body.youtube-body { background-color: rgba(0, 0, 0, 0); }",
  ".overlay-hidden { display: none }"
]

export default class YouTube extends React.Component {
  constructor(props) {
    super(props);

    this.playerElement = null;
    this.player = null;
    this.playerRef = React.createRef();
    this.currentId = null;

    this.state = {
      stopped: true,
      paused: true,
      loading: true,
      events: [],
      api: null,
      videoId: 'M7lc1UVf-VE',
    };
  }

  handleData(d) {
    let data = null;

    try {
      data = JSON.parse(d);
    } catch(e) {
      console.log("failed to deserialize message");
      return;
    }

    switch (data.type) {
      case "youtube/current":
        switch (data.event.type) {
          case "play":
            if (this.currentId !== data.event.video_id) {
              let videoId = data.event.video_id;
              this.player.loadVideoById({videoId});
              this.player.seekTo(data.event.elapsed);
              this.player.playVideo();
              this.currentId = data.event.video_id;
            } else if (this.state.paused || Math.abs(data.event.elapsed - this.player.getCurrentTime()) > 2) {
              this.player.seekTo(data.event.elapsed);
              this.player.playVideo();
            }

            this.setState({
              stopped: false,
              paused: false,
            });
            break;
          case "pause":
            this.player.pauseVideo();

            this.setState({
              stopped: false,
              paused: true,
            });
            break;
          case "stop":
            this.player.pauseVideo();

            this.setState({
              stopped: true,
              paused: false,
            });
            break;
          default:
            break;
        }

        break;
      case "youtube/volume":
        this.player.setVolume(data.volume);
        break;
      case "song/progress":
        return;
      default:
        return;
    }
  }

  setupPlayer() {
    if (!this.playerRef.current) {
      throw new Error("Reference to player is not available");
    }

    this.player = new YT.Player(this.playerRef.current, {
      width: '100%',
      height: '100%',
      autoplay: false,
      events: {
        onReady: () => {
          this.setState({
            loading: false,
          });
        },
      }
    });
  }

  componentDidMount() {
    window.onYouTubeIframeAPIReady = () => {
      this.setupPlayer();
    };

    var tag = document.createElement('script');
    tag.src = "https://www.youtube.com/iframe_api";
    tag.setAttribute("x-youtube", "");

    var firstScriptTag = document.getElementsByTagName('script')[0];
    firstScriptTag.parentNode.insertBefore(tag, firstScriptTag);
  }

  componentWillMount() {
    document.body.classList.add('youtube-body');
  }

  componentWillUnmount() {
    let scripts = document.getElementsByTagName('script');

    for (var script of scripts) {
      if (scripts.hasAttribute("x-youtube")) {
        script.parentNode.removeChild(script);
      }
    }

    delete window.onYouTubeIframeAPIReady;
    document.body.classList.remove('youtube-body');
  }

  render() {
    var loading = null;
    var ws = null;

    var playerStyle = {};

    if (this.state.loading) {
      loading = (
        <div className="player-loading">
          <Spinner />
        </div>
      );
    } else {
      ws = <Websocket url={websocketUrl("ws/youtube")} onMessage={this.handleData.bind(this)} />;
    }

    var noVideo = null;

    if (this.state.stopped) {
      playerStyle.display = "none";
      noVideo = (
        <div className="overlay-hidden youtube-not-loaded p-4 container">
          <h1>No Video Loaded</h1>

          <p>
            If you want to embed this into OBS, please add the following Custom CSS:
          </p>

          <pre className="youtube-not-loaded-obs"><code>
            {OBS_CSS.join("\n")}
          </code></pre>
        </div>
      );
    }

    return (
      <div id="youtube">
        {ws}

        {noVideo}

        {loading}

        <div className="youtube-container" style={playerStyle}>
          <div ref={this.playerRef} className="youtube-embedded"></div>
        </div>
      </div>
    );
  }
}