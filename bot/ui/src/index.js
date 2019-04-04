import "./index.scss";
import * as utils from "./utils.js";
import {Api} from "./api.js";
import React from "react";
import ReactDOM from "react-dom";
import { BrowserRouter as Router, Route, Link, withRouter} from "react-router-dom";
import Websocket from "react-websocket";
import {Container, Row, Col, Navbar, Nav} from "react-bootstrap";
import Authentication from "./components/Authentication.js";
import Devices from "./components/Devices.js";
import AfterStreams from "./components/AfterStreams.js";
import { createBrowserHistory } from "history";
import '@fortawesome/fontawesome-free-solid'

const history = createBrowserHistory()

class CurrentSong extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let requestBy = null;

    if (this.props.requestBy !== null) {
      requestBy = (
        <span class="request">
          <span class="request-title">request by</span>
          <span class="request-by">{this.props.requestBy}</span>
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
      width: `${utils.percentage(this.props.elapsed, this.props.duration)}%`,
    };

    let stateClasses = "state";

    if (this.props.isPlaying) {
      stateClasses += " state-playing";
    } else {
      stateClasses += " state-paused";
    }

    let trackName = "Unknown Track";

    if (this.props.track) {
      trackName = this.props.track.name;
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
            {requestBy}
          </div>

          <div className="artist">
            <span className="artist-name">{artistName}</span>
          </div>

          <div className="progress">
            <span className="timer">
                <span className="elapsed">{utils.formatDuration(this.props.elapsed)}</span>
                <span>/</span>
                <span className="duration">{utils.formatDuration(this.props.duration)}</span>
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

class Overlay extends React.Component {
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
          update.track = data.track;
          update.artist = utils.pickArtist(data.track.artists);
          update.albumArt = utils.pickAlbumArt(data.track.album.images, 64);
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
      <div>
        <Websocket url={utils.websocketUrl("ws/overlay")} onMessage={this.handleData.bind(this)} />

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

const RouteLayout = withRouter(props => <Layout {...props} />)

class AfterStreamsPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <Row>
          <Col>
            <AfterStreams api={this.api} />
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

class IndexPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <Row>
          <Col>
            <h4>Administration</h4>

            <p>
            Congratulations on getting <b>setmod</b> running!
            </p>

            <p>
              If you need more help, go to the <a href="https://github.com/udoprog/setmod">README</a>.
            </p>
          </Col>
        </Row>

        <Row>
          <Col lg="6">
            <Authentication api={this.api} />
          </Col>

          <Col lg="6">
            <Devices api={this.api} />
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

function Layout(props) {
  let path = props.location.pathname;

  return (
    <div>
      <Navbar bg="light" expand="sm">
        <Navbar.Brand href="https://github.com/udoprog/setmod">setmod</Navbar.Brand>
        <Navbar.Toggle aria-controls="basic-navbar-nav" />

        <Navbar.Collapse id="basic-navbar-nav">
          <Nav className="mr-auto">
            <Nav.Link as={Link} active={path === "/"} to="/">Home</Nav.Link>
            <Nav.Link as={Link} active={path === "/after-streams"} to="/after-streams">After Streams</Nav.Link>
            <Nav.Link as={Link} active={path === "/overlay"} to="/overlay" target="overlay">Overlay</Nav.Link>
          </Nav>
        </Navbar.Collapse>
      </Navbar>,

      <Container>
        {props.children}
      </Container>
    </div>
  );
}

function AppRouter() {
  return (
    <Router history={history}>
      <Route path="/" exact component={IndexPage} />
      <Route path="/after-streams" exact component={AfterStreamsPage} />
      <Route path="/overlay/" component={Overlay} />
    </Router>
  );
}

ReactDOM.render(<AppRouter />, document.getElementById("index"));