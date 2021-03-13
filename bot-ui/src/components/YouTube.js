import React from "react";
import {websocketUrl} from "../utils.js";
import Websocket from "react-websocket";
import Loading from 'shared-ui/components/Loading';

const OBS_CSS = [
  "body.youtube-body { background-color: rgba(0, 0, 0, 0); }",
  ".overlay-hidden { display: none }"
]

const UNSTARTED = -1;
const ENDED = 0;
const PLAYING = 1;
const PAUSED = 2;
const BUFFERING = 3;
const VIDEO_CUED = 5;
const SUGGESTED_QUALITY = "hd720";

export default class YouTube extends React.Component {
  constructor(props) {
    super(props);

    this.playerElement = null;
    this.player = null;
    this.playerRef = React.createRef();

    this.state = {
      playing: false,
      stopped: true,
      loading: true,
      events: [],
      api: null,
      videoId: null,
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
            let update = { stopped: false };

            if (!this.state.playing) {
              this.player.playVideo();
              update.playing = true;
            }

            if (this.state.videoId !== data.event.video_id) {
              let videoId = data.event.video_id;
              this.player.loadVideoById({videoId, suggestedQuality: SUGGESTED_QUALITY});
              this.player.seekTo(data.event.elapsed, true);
              update.videoId = data.event.video_id;
            } else {
              // We are a bit out of sync.
              if (Math.abs(data.event.elapsed - this.player.getCurrentTime()) > 2) {
                this.player.seekTo(data.event.elapsed, true);
              }
            }

            this.setState(update);
            break;
          case "pause":
            if (this.state.playing) {
              this.player.pauseVideo();
            }

            this.setState({ playing: false, stopped: false });
            break;
          case "stop":
            if (this.state.playing) {
              this.player.stopVideo();
            }

            this.setState({ playing: false, stopped: true, videoId: null });
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
      width: 1280,
      height: 720,
      autoplay: false,
      events: {
        onReady: () => {
          if (this.state.playing) {
            this.player.playVideo();
          }
  
          this.setState({
            loading: false,
          });
        },
        onPlaybackQualityChange: e => {
        },
        onStateChange: event => {
          if (event.data === -1) {
            if (this.state.playing) {
              this.player.playVideo();
            }
          }
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
    var ws = null;
    var playerStyle = {};

    if (!this.state.loading) {
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
        <Loading isLoading={this.state.loading} />

        <div className="youtube-container" style={playerStyle}>
          <div ref={this.playerRef} className="youtube-embedded"></div>
        </div>
      </div>
    );
  }
}
