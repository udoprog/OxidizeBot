import "./index.scss";
import '@fortawesome/fontawesome-free-solid'
import React from "react";
import ReactDOM from "react-dom";
import {BrowserRouter as Router, Route, Link, withRouter} from "react-router-dom";
import {Container, Row, Col, Navbar, Nav, NavDropdown, Alert} from "react-bootstrap";
import logo from "./logo.png";

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
          </Nav>
        </Navbar.Collapse>
      </Navbar>

      {props.children}
    </Container>
  );
}

const RouteLayout = withRouter(props => <Layout {...props} />)

class IndexPage extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <RouteLayout>
        <Row>
          <Col>
            Hello World
          </Col>
        </Row>
      </RouteLayout>
    );
  }
}

function AppRouter() {
  return (
    <Router>
      <Route path="/" exact component={IndexPage} />
    </Router>
  );
}

ReactDOM.render(<AppRouter />, document.getElementById("index"));