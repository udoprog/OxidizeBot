import React from "react";
import { Link } from "react-router-dom";
import { withRouter } from "react-router-dom";
import { Container, Navbar, Nav } from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import CurrentUser from "./CurrentUser.js";
import logo from "../assets/logo.png";

function links(props) {
  let links = [];

  links.push(
    <Nav.Item key="players">
      <Nav.Link as={Link} active={props.match.path === "/players"} to="/players">
        <FontAwesomeIcon icon="music" /> Players
      </Nav.Link>
    </Nav.Item>
  );

  if (props.match.path === "/player/:id") {
    links.push(
      <Nav.Item key="player-self">
        <Nav.Link as={Link} active={true} to={props.location.pathname}>
          {props.match.params.id}
        </Nav.Link>
      </Nav.Item>
    );
  }

  return links;
}

class Layout extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let navLinks = links(this.props);

    return [
      <div key="navigation" id="navbar">
        <Container>
          <Navbar key="nav" expand="sm" className="mb-3" variant="dark">
            <Navbar.Brand>
              <img src={logo} alt="Logo" width="32" height="32"></img>
            </Navbar.Brand>
            <Navbar.Toggle aria-controls="basic-navbar-nav" />
            <Navbar.Collapse id="basic-navbar-nav">
              <Nav>
                <Nav.Item>
                  <Nav.Link as={Link} active={this.props.match.path === "/"} to="/">
                    <FontAwesomeIcon icon="home" /> OxidizeBot
                  </Nav.Link>
                </Nav.Item>
                {navLinks}
              </Nav>
            </Navbar.Collapse>

            <CurrentUser />
          </Navbar>
        </Container>
      </div>,

      <Container key="content" id="content" className="mb-3">
        {this.props.children}
      </Container>,

      <Container key="footer" id="footer" className="pt-2 pb-2">
        <span className="highlight">setbac.tv</span> is operated by <a href="https://twitch.tv/setbac">setbac</a><br />
        <Link to="/">Start Page</Link> &ndash; <Link to="/privacy">Privacy Policy</Link>
      </Container>
    ];
  }
}

export const RouteLayout = withRouter(props => <Layout {...props} />)