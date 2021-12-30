import React from "react";
import { Link, Outlet, useLocation } from "react-router-dom";
import { Container, Navbar, Nav } from "react-bootstrap";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import CurrentUser from "./CurrentUser.js";
import logo32 from "../assets/logo-32px.png";

function links() {
  let loc = useLocation();

  let links = [];

  links.push(
    <Nav.Item key="help">
      <Nav.Link as={Link} active={loc.pathname=== "/help"} to="/help">
        <FontAwesomeIcon icon="question" />&nbsp;Help
      </Nav.Link>
    </Nav.Item>
  );

  links.push(
    <Nav.Item key="connections">
      <Nav.Link as={Link} active={loc.pathname=== "/connections"} to="/connections">
        <FontAwesomeIcon icon="globe" />&nbsp;My&nbsp;Connections
      </Nav.Link>
    </Nav.Item>
  );

  links.push(
    <Nav.Item key="playlists">
      <Nav.Link as={Link} active={loc.pathname=== "/playlists" || loc.pathname=== "/player/:id"} to="/playlists">
        <FontAwesomeIcon icon="music" />&nbsp;Playlists
      </Nav.Link>
    </Nav.Item>
  );

  console.log("HELLO");

  return links;
}

export default function Layout() {
  let navLinks = links();

  return <>
    <div key="navigation" id="navbar">
      <Navbar key="nav" expand="sm" className="mb-3" bg="light">
        <Container>
          <Navbar.Brand>
            <Link to="/">
              <img src={logo32} alt="Logo" width="32" height="32"></img>
            </Link>
          </Navbar.Brand>

          <Navbar.Collapse>
            <Nav>
              {navLinks}
            </Nav>

            <Nav className="ml-auto">
              <Nav.Item className="nav-link">
                <CurrentUser />
              </Nav.Item>
            </Nav>
          </Navbar.Collapse>

          <Navbar.Toggle aria-controls="basic-navbar-nav" />
        </Container>
      </Navbar>
    </div>

    <Container key="content" id="content" className="mb-3">
      <Outlet />
    </Container>

    <Container key="footer" id="footer" className="pt-2 pb-2">
      <span className="oxi-highlight">setbac.tv</span> is built and operated with ♥ by <a href="https://twitch.tv/setbac">setbac</a> (<a href="https://github.com/udoprog" title="Github"><FontAwesomeIcon icon={['fab', 'github']} /></a> - <a href="https://twitter.com/udoprog" title="Twitter"><FontAwesomeIcon icon={['fab', 'twitter']} /></a> - <a href="https://twitch.com/setbac"><FontAwesomeIcon icon={['fab', 'twitch']} title="Twitch" /></a>)<br />
      Come join my <a href="https://discord.gg/v5AeNkT">Discord Community</a> if you want to participate in this Project<br />
      <Link to="/">Start Page</Link> &ndash; <Link to="/privacy">Privacy Policy</Link>
    </Container>
  </>;
}
