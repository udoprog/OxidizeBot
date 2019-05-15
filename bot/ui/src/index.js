import "./index.scss";
import * as utils from "./utils.js";
import {Api} from "./api.js";
import React from "react";
import ReactDOM from "react-dom";
import { BrowserRouter as Router, Route, Link, withRouter} from "react-router-dom";
import {Container, Row, Col, Navbar, Nav, NavDropdown} from "react-bootstrap";
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
import YouTube from "./components/YouTube";

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

class SettingsPage extends React.Component {
  constructor(props) {
    super(props);
    this.api = new Api(utils.apiUrl());
  }

  render() {
    return (
      <RouteLayout>
        <Row>
          <Col>
            <Settings api={this.api} />
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

class AliasesPage extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      current: null,
    };

    this.api = new Api(utils.apiUrl());
  }

  componentWillMount() {
    this.api.current().then(current => {
      this.setState({current});
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

    return (
      <RouteLayout>
        <Row>
          <Col>
            <Aliases current={this.state.current} api={this.api} />
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

class CommandsPage extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      current: null,
    };

    this.api = new Api(utils.apiUrl());
  }

  componentWillMount() {
    this.api.current().then(current => {
      this.setState({current});
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

    return (
      <RouteLayout>
        <Row>
          <Col>
            <Commands current={this.state.current} api={this.api} />
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

class PromotionsPage extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      current: null,
    };

    this.api = new Api(utils.apiUrl());
  }

  componentWillMount() {
    this.api.current().then(current => {
      this.setState({current});
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

    return (
      <RouteLayout>
        <Row>
          <Col>
            <Promotions current={this.state.current} api={this.api} />
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

            <NavDropdown title="Internal">
              <NavDropdown.Item as={Link} active={path === "/settings"} to="/settings">Settings</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/import-export"} to="/import-export">Import / Export</NavDropdown.Item>
            </NavDropdown>

            <NavDropdown title="Chat">
              <NavDropdown.Item as={Link} active={path === "/after-streams"} to="/after-streams">After Streams</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/aliases"} to="/aliases">Aliases</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/commands"} to="/commands">Commands</NavDropdown.Item>
              <NavDropdown.Item as={Link} active={path === "/promotions"} to="/promotions">Promotions</NavDropdown.Item>
            </NavDropdown>

            <Nav.Link as={Link} active={path === "/overlay"} to="/overlay" target="overlay">Overlay</Nav.Link>

            <NavDropdown title="Experimental">
              <NavDropdown.Item as={Link} active={path === "/youtube"} to="/youtube" target="youtube">YouTube Player</NavDropdown.Item>
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
      <Route path="/import-export" exact component={ImportExportPage} />
      <Route path="/aliases" exact component={AliasesPage} />
      <Route path="/commands" exact component={CommandsPage} />
      <Route path="/promotions" exact component={PromotionsPage} />
      <Route path="/overlay/" component={Overlay} />
      <Route path="/youtube" component={YouTube} />
    </Router>
  );
}

ReactDOM.render(<AppRouter />, document.getElementById("index"));