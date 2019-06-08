import "./index.scss";
import * as utils from "./utils.js";
import {Api} from "./api.js";
import React from "react";
import ReactDOM from "react-dom";
import {BrowserRouter as Router, Route, Link, withRouter} from "react-router-dom";
import {Container, Row, Col, Navbar, Nav, NavDropdown, Alert} from "react-bootstrap";
import Authentication from "./components/Authentication.js";
import Devices from "./components/Devices.js";
import AfterStreams from "./components/AfterStreams.js";
import Overlay from "./components/Overlay.js";
import Settings from "./components/Settings.js";
import ImportExport from "./components/ImportExport.js";
import Commands from "./components/Commands.js";
import '@fortawesome/fontawesome-free-solid'
import Promotions from "./components/Promotions";
import Aliases from "./components/Aliases";
import Themes from "./components/Themes";
import YouTube from "./components/YouTube";
import Authorization from "./components/Authorization";

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

const SPOTIFY_CONFIG = "secrets/oauth2/spotify/config";

class ConfigurationPrompt extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      configured: true,
      loading: false,
      error: null,
    }
  }

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.list();
  }

  list() {
    this.setState({
      loading: true,
    });

    this.props.api.settings({keyFilter: this.props.keyFilter})
      .then(settings => {
        this.setState({
          configured: settings.every(s => s.value !== null),
          loading: false,
        })
      },
      e => {
        this.setState({
          error: e,
          loading: false,
        })
      });
  }

  render() {
    if (this.state.configured) {
      return null;
    }

    let error = null;

    if (this.state.error) {
      error = <Alert key="error" variant="warning">{this.state.error}</Alert>;
    }

    return [
      error,

      <Row key="help">
        <Col>
          {this.props.children}
        </Col>
      </Row>,

      <Row key="settings">
        <Col>
          <Settings
            api={this.props.api}
            keyFilter={this.props.keyFilter}
            filterable={false} />
        </Col>
      </Row>,
    ];
  }
}

class SettingsPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <h2>Settings</h2>

        <Row>
          <Col>
            <Settings api={this.api} filterable={true} />
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

class ImportExportPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <Row>
          <Col>
            <ImportExport api={this.api} />
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

class AuthorizedPage extends React.Component {
  constructor(props, page) {
    super(props);

    this.state = {
      current: null,
    };

    this.api = new Api(utils.apiUrl());
    this.page = page;
  }

  componentWillMount() {
    this.api.current().then(current => {
      if (current.channel) {
        this.setState({current});
      }
    });
  }

  render() {
    if (!this.state.current) {
      return (
        <RouteLayout>
          <div className="loading">
            Loading Current User
            <utils.Spinner />
          </div>
        </RouteLayout>
      );
    }

    const children = React.Children.map(this.props.children, child => {
      return React.cloneElement(child, { api: this.api, current: this.state.current });
    });

    return (
      <RouteLayout>
        <Row>
          <Col>{children}</Col>
        </Row>
      </RouteLayout>
    );
  }
}

class IndexPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
    this.state = {
      version: null,
    };
  }

  componentDidMount() {
    this.api.version().then(version => {
      this.setState({version});
    });
  }

  render() {
    let version = <utils.Spinner />;
    let newVersion = null;

    if (this.state.version) {
      version = this.state.version.version;
      let latest = this.state.version.latest;

      if (latest && latest.version != version) {
        let dl = null;

        if (latest.asset) {
          dl = (
            <div>
              Download it from:&nbsp;
              <b><a href={latest.asset.download_url}>{latest.asset.name}</a></b>
            </div>
          );
        } else {
          let releases_url = `https://github.com/udoprog/setmod/releases/${latest.version}`;

          dl = (
            <div>
              Download is not ready <em>just yet</em>, but you can find it later at:&nbsp;
              <b><a href={releases_url}>GitHub Releases</a></b>
            </div>
          )
        }

        newVersion = (
          <Alert variant="info">
            <b>Version {latest.version} of SetMod is available!</b>
            {dl}
          </Alert>
        );
      }
    }

    return (
      <RouteLayout>
        <Row>
          <Col>
            <p>
              Congratulations on getting <b>SetMod {version}</b> running!
            </p>
            {newVersion}
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

        <ConfigurationPrompt api={this.api} keyFilter={[SPOTIFY_CONFIG]}>
          <h4><b>Action Required</b>: OAuth 2.0 Configuration for Spotify</h4>

          <p>
            You will have <a href="https://developer.spotify.com/dashboard/">register an application with Spotify</a>
          </p>

          <p>
            You must configure the following redirect URL:<br />
            <code>http://localhost:12345/redirect</code>
          </p>
        </ConfigurationPrompt>
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
            <Nav.Link as={Link} active={path === "/"} to="/">
              Home
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/settings"} to="/settings">
              Settings
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/authorization"} to="/authorization">
              Authorization
            </Nav.Link>

            <NavDropdown title="Chat">
              <NavDropdown.Item as={Link} active={path === "/after-streams"} to="/after-streams">After Streams</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/aliases"} to="/aliases">Aliases</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/commands"} to="/commands">Commands</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/promotions"} to="/promotions">Promotions</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/themes"} to="/themes">Themes</NavDropdown.Item>
            </NavDropdown>

            <NavDropdown title="Misc">
              <NavDropdown.Item as={Link} active={path === "/import-export"} to="/import-export">
                Import / Export
              </NavDropdown.Item>
            </NavDropdown>

            <NavDropdown title="Experimental">
              <NavDropdown.Item as={Link} active={path === "/overlay"} to="/overlay" target="overlay">
                Overlay
              </NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/youtube"} to="/youtube" target="youtube">
                YouTube Player
              </NavDropdown.Item>
            </NavDropdown>
          </Nav>
        </Navbar.Collapse>
      </Navbar>

      <Container className="content">
        {props.children}
      </Container>
    </div>
  );
}

function AppRouter() {
  return (
    <Router>
      <Route path="/" exact component={IndexPage} />
      <Route path="/after-streams" exact component={AfterStreamsPage} />
      <Route path="/settings" exact component={SettingsPage} />
      <Route path="/authorization" exact component={props => (
        <AuthorizedPage><Authorization {...props} /></AuthorizedPage>
      )} />
      <Route path="/import-export" exact component={ImportExportPage} />
      <Route path="/aliases" exact render={props => (
        <AuthorizedPage><Aliases {...props} /></AuthorizedPage>
      )} />
      <Route path="/commands" exact render={props => (
        <AuthorizedPage><Commands {...props} /></AuthorizedPage>
      )} />
      <Route path="/promotions" exact render={props => (
        <AuthorizedPage><Promotions {...props} /></AuthorizedPage>
      )} />
      <Route path="/themes" exact render={props => (
        <AuthorizedPage><Themes {...props} /></AuthorizedPage>
      )} />
      <Route path="/overlay/" component={Overlay} />
      <Route path="/youtube" component={YouTube} />
    </Router>
  );
}

ReactDOM.render(<AppRouter />, document.getElementById("index"));