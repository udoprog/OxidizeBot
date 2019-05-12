import Websocket from "react-websocket";
import React from "react";
import {formatDuration, percentage, pickArtist, pickAlbumArt, websocketUrl} from "../utils.js";

/**
 * Pick the image best suited for album art.
 */
export function pickYouTubeAlbumArt(thumbnails, smaller) {
  let smallest = null;

  for (let key in thumbnails) {
    let thumbnail = thumbnails[key];

    if (smallest === null) {
      smallest = thumbnail;
      continue;
    }

    if (smallest.width > thumbnail.width) {
      smallest = thumbnail;
    }
  }

  if (smallest.width > smaller) {
    let factor = smaller / smallest.width;
    smallest.width *= factor;
    smallest.height *= factor;
  }

  return smallest;
}

class CurrentSong extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let requestBy = null;

    if (this.props.requestBy !== null) {
      requestBy = (
        <span className="request">
          <span className="request-by">request by</span>
          <span className="request-user">{this.props.requestBy}</span>
        </span>
      );
    }

    let state = null;
    let albumArt = null;

    if (this.props.albumArt) {
      state = <div className={stateClasses}></div>;

      albumArt = (
        <img className="album-art"
          width={this.props.albumArt.width}
          height={this.props.albumArt.height}
          src={this.props.albumArt.url} />
      );
    }

    let progressBarStyle = {
      width: `${percentage(this.props.elapsed, this.props.duration)}%`,
    };

    let stateClasses = "state";

    if (this.props.isPlaying) {
      stateClasses += " state-playing";
    } else {
      stateClasses += " state-paused";
    }

    let trackName = "Unknown Track";

    if (this.props.track) {
      trackName = this.props.track;
    }

    let artistName = "Unknown Artist";

    if (this.props.artist) {
      artistName = this.props.artist.name;
    }

    return (
      <div id="current-song">
        <div className="album">
          {state}
          {albumArt}
        </div>

        <div className="info">
          <div className="track">
            <div className="track-name">{trackName}</div>
          </div>

          <div className="artist">
            <span className="artist-name">{artistName}</span>
            {requestBy}
          </div>

          <div className="progress">
            <span className="timer">
              <span className="elapsed">{formatDuration(this.props.elapsed)}</span>
              <span>/</span>
              <span className="duration">{formatDuration(this.props.duration)}</span>
            </span>

            <div
              className="progress-bar"
              role="progressbar"
              aria-valuenow="0"
              aria-valuemin="0"
              aria-valuemax="100"
              style={progressBarStyle} />
          </div>
        </div>
      </div>
    );
  }
}

export default class Overlay extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      artist: "Unknown",
      track: null,
      requestBy: null,
      albumArt: null,
      elapsed: 0,
      duration: 0,
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
      case "song/current":
        let update = {
          requestBy: data.user,
          elapsed: data.elapsed,
          duration: data.duration,
        };

        if (data.track) {
          switch (data.track.type) {
            case "spotify":
              let track = data.track.track;
              update.track = track.name;
              update.artist = pickArtist(track.artists);
              update.albumArt = pickAlbumArt(track.album.images, 64);
              break;
            case "youtube":
              let video = data.track.video;

              if (video.snippet) {
                update.artist = {
                  name: `channel: ${video.snippet.channelTitle}`,
                };
                update.track = video.snippet.title;
                update.albumArt = pickYouTubeAlbumArt(video.snippet.thumbnails, 64);
              } else {
                update.track = null;
                update.albumArt = null;
                update.artist = null;
              }

              break;
            default:
              break;
          }
        }

        this.setState(update);
        break;
      case "song/progress":
        this.setState({
          elapsed: data.elapsed,
          duration: data.duration,
        });

        break;
    }
  }

  render() {
    return (
      <div id="overlay">
        <Websocket url={websocketUrl("ws/overlay")} onMessage={this.handleData.bind(this)} />

        <CurrentSong
          artist={this.state.artist}
          track={this.state.track}
          requestBy={this.state.requestBy}
          albumArt={this.state.albumArt}
          elapsed={this.state.elapsed}
          duration={this.state.duration}
        />
      </div>
    );
  }
}