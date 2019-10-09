import "./index.scss";
import * as utils from "./utils.js";
import {Api} from "./api.js";
import React from "react";
import ReactDOM from "react-dom";
import {BrowserRouter as Router, Route, Link, withRouter} from "react-router-dom";
import {Container, Row, Col, Navbar, Nav, NavDropdown, Alert} from "react-bootstrap";
import Connections from "./components/Connections.js";
import Devices from "./components/Devices.js";
import AfterStreams from "./components/AfterStreams.js";
import Overlay from "./components/Overlay.js";
import Settings from "./components/Settings.js";
import Cache from "./components/Cache";
import Modules from "./components/Modules.js";
import ImportExport from "./components/ImportExport.js";
import Commands from "./components/Commands.js";
import '@fortawesome/fontawesome-free-solid'
import Promotions from "./components/Promotions";
import Aliases from "./components/Aliases";
import Themes from "./components/Themes";
import YouTube from "./components/YouTube";
import Chat from "./components/Chat";
import Authorization from "./components/Authorization";
import ConfigurationPrompt from "./components/ConfigurationPrompt";
import * as semver from "semver";
import logo from "./logo.png";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { InlineLoading, Loading, Error } from 'shared-ui/components';

/**
 * Required spotify configuration.
 */
const SECRET_KEY_CONFIG = "remote/secret-key";
const RouteLayout = withRouter(props => <Layout {...props} />)

class AfterStreamsPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <AfterStreams api={this.api} />
      </RouteLayout>
    );
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
        <h1 className="oxi-page-title">Settings</h1>

        <Settings group={true} api={this.api} filterable={true} {...this.props} />
      </RouteLayout>
    );
  }
}

class CachePage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <h1 className="oxi-page-title">Cache</h1>

        <Cache api={this.api} {...this.props} />
      </RouteLayout>
    );
  }
}

class ModulesPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <Modules api={this.api} {...this.props} />
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
        <ImportExport api={this.api} {...this.props} />
      </RouteLayout>
    );
  }
}

class AuthorizedPage extends React.Component {
  constructor(props, page) {
    super(props);

    this.state = {
      current: null,
      error: null,
    };

    this.api = new Api(utils.apiUrl());
    this.page = page;
  }

  async componentDidMount() {
    try {
      let current = await this.api.current();

      if (current.channel) {
        this.setState({current});
      }
    } catch (e) {
      this.setState({error: `Failed to get current user: ${e}`})
    }
  }

  render() {
    if (this.state.error) {
      return <RouteLayout><Error error={this.state.error} /></RouteLayout>;
    }

    if (!this.state.current) {
      return <RouteLayout><Loading>Loading user information</Loading></RouteLayout>;
    }

    const children = React.Children.map(this.props.children, child => {
      return React.cloneElement(child, { api: this.api, current: this.state.current });
    });

    return (
      <RouteLayout>{children}</RouteLayout>
    );
  }
}

function HeaderAction(props) {
  let link = {};
  let icon = {};

  if (!!props.icon) {
    icon.icon = props.icon;
  }

  if (!!props.to) {
    link.to = props.to;
  }

  return <Link className="oxi-header-action" {...link}><FontAwesomeIcon {...icon} />&nbsp;{props.children}</Link>;
}

class IndexPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());

    let q = new URLSearchParams(props.location.search);

    this.state = {
      version: null,
      receivedKey: q.get("received-key") === "true",
    };
  }

  componentDidMount() {
    this.api.version().then(version => {
      this.setState({version});
    });
  }

  /**
   * Get default version information.
   */
  defaultVersionInfo(version) {
    return (
      <Alert variant="info" style={{textAlign: "center"}}>
        You're running the latest version of <b>OxidizeBot</b> (<b>{version}</b>).
      </Alert>
    );
  }

  /**
   * Get information on new versions available, or the current version of the bot.
   */
  renderVersionInfo() {
    let version = <InlineLoading />;
    let latest = null;

    if (this.state.version) {
      version = this.state.version.version;
      latest = this.state.version.latest;
    }

    if (!latest || !semver.valid(latest.version)) {
      return this.defaultVersionInfo(version);
    }

    if (!semver.gt(latest.version, version) || !latest.asset) {
      return this.defaultVersionInfo(version);
    }

    return (
      <Alert variant="warning" className="center">
        <div className="mb-2" style={{fontSize: "150%"}}>
          OxidizeBot <b>{latest.version}</b> is available (current: <b>{version}</b>).
        </div>

        <div>
          Download link:&nbsp;
          <a href={latest.asset.download_url}>{latest.asset.name}</a>
        </div>
      </Alert>
    );
  }

  render() {
    let receivedKey = null;

    if (this.state.receivedKey) {
      receivedKey = <Alert variant="info" className="center">
        <FontAwesomeIcon icon="key" /> Received new <b>Secret Key</b> from setbac.tv
      </Alert>;
    }

    let versionInfo = this.renderVersionInfo();

    return (
      <RouteLayout>
        {versionInfo}

        <Row>
          <Col lg="6">
            <h4>
              Connections
              <HeaderAction to="/modules/remote" icon="wrench">Configure</HeaderAction>
            </h4>

            {receivedKey}

            <Connections api={this.api} />
          </Col>

          <Col lg="6">
            <h4>Devices</h4>

            <Devices api={this.api} />
          </Col>
        </Row>

        <ConfigurationPrompt api={this.api} hideWhenConfigured={true} filter={{key: [SECRET_KEY_CONFIG]}}>
          <h4><b>Action Required</b>: Configure your connection to <a href="https://setbac.tv">setbac.tv</a></h4>

          Go to <a href="https://setbac.tv/connections">your connections</a> and login using Twitch.

          Generate a new key and configure it below.
        </ConfigurationPrompt>
      </RouteLayout>
    );
  }
}

function Layout(props) {
  let path = props.location.pathname;

  return (
    <Container className="content">
      <Navbar bg="light" expand="sm" className="mb-3">
        <Navbar.Brand>
          <img src={logo} alt="Logo" width="32" height="32"></img>
        </Navbar.Brand>
        <Navbar.Toggle aria-controls="basic-navbar-nav" />

        <Navbar.Collapse id="basic-navbar-nav">
          <Nav>
            <Nav.Link as={Link} active={path === "/"} to="/">
              Home
            </Nav.Link>
            <Nav.Link as={Link} active={path.startsWith("/modules")} to="/modules">
              Modules
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/authorization"} to="/authorization">
              Authorization
            </Nav.Link>

            <NavDropdown title="Chat">
              <NavDropdown.Item as={Link} active={path === "/after-streams"} to="/after-streams">
                After Streams
              </NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/aliases"} to="/aliases">
                Aliases
              </NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/commands"} to="/commands">
                Commands
              </NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/promotions"} to="/promotions">
                Promotions
              </NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/themes"} to="/themes">
                Themes
              </NavDropdown.Item>
            </NavDropdown>

            <NavDropdown title="Advanced">
              <NavDropdown.Item as={Link} active={path === "/settings"} to="/settings">
                Settings
              </NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/cache"} to="/cache">
                Cache
              </NavDropdown.Item>
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
              <NavDropdown.Item as={Link} active={path === "/chat"} to="/chat" target="chat">
                Chat
              </NavDropdown.Item>
            </NavDropdown>
          </Nav>
        </Navbar.Collapse>
      </Navbar>

      {props.children}
    </Container>
  );
}

function AppRouter() {
  return (
    <Router>
      <Route path="/" exact component={IndexPage} />
      <Route path="/after-streams" exact component={AfterStreamsPage} />
      <Route path="/settings" exact component={SettingsPage} />
      <Route path="/cache" exact component={CachePage} />
      <Route path="/modules" component={ModulesPage} />
      <Route path="/authorization" exact component={props => (
        <AuthorizedPage><Authorization {...props} /></AuthorizedPage>
      )} />
      <Route path="/import-export" component={ImportExportPage} />
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
      <Route path="/chat" component={Chat} />
    </Router>
  );
}

ReactDOM.render(<AppRouter />, document.getElementById("index"));