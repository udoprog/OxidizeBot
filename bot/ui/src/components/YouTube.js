import {Spinner} from "../utils.js";
import React from "react";
import {Button, Table} from "react-bootstrap";
import {websocketUrl} from "../utils.js";
import Websocket from "react-websocket";

export default class YouTube extends React.Component {
  constructor(props) {
    super(props);

    this.playerElement = null;
    this.player = null;
    this.playerRef = React.createRef();
    this.currentId = null;

    this.state = {
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
              let startSeconds = data.event.elapsed;
              let endSeconds = data.event.elapsed + data.event.duration;

              this.player.loadVideoById({
                videoId,
                startSeconds,
                endSeconds,
              });

              this.currentId = data.event.video_id;
            } else if (this.state.paused || Math.abs(data.event.elapsed - this.player.getCurrentTime()) > 5) {
              this.player.seekTo(data.event.elapsed);
              this.player.playVideo();
            }

            this.setState({
              paused: false,
            });
            break;
          case "pause":
            this.player.pauseVideo();

            this.setState({
              paused: true,
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
      autoplay: false,
      playerVars: {'autoplay': 0},
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

  componentWillUnmount() {
    let scripts = document.getElementsByTagName('script');

    for (var script of scripts) {
      if (scripts.hasAttribute("x-youtube")) {
        script.parentNode.removeChild(script);
      }
    }

    delete window.onYouTubeIframeAPIReady;
  }

  render() {
    var loading = null;
    var ws = null;

    var noVideoStyle = {};
    var playerStyle = {};

    if (this.state.loading) {
      loading = (
        <div className="player-loading">
          Loading Player
          <Spinner />
        </div>
      );
    } else {
      ws = <Websocket url={websocketUrl("ws/overlay")} onMessage={this.handleData.bind(this)} />;
    }

    if (this.state.paused) {
      playerStyle.display = "none";
    } else {
      noVideoStyle.display = "none";
    }

    return (
      <div>
        {ws}

        {loading}

        <div style={noVideoStyle} className="player-not-loaded"><em>No Video Loaded</em></div>

        <div style={playerStyle}>
          <div ref={this.playerRef} className="player-embedded"></div>
        </div>
      </div>
    );
  }
}