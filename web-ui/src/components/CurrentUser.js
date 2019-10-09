import React from "react";
import { Navbar, NavDropdown, Form, Button, Dropdown, ButtonGroup } from "react-bootstrap";
import { Link } from "react-router-dom";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { api, currentUser, cameFromBot } from "../globals.js";
import twitchLogo from "../assets/twitch.png";
import logo from "../assets/logo.png";

export default class CurrentUser extends React.Component {
  constructor(props) {
    super(props);
  }

  async login() {
    let result = await api.authLogin();
    location.href = result.auth_url;
  }

  async logout() {
    let result = await api.authLogout();
    location.reload();
  }

  backToBot() {
    document.location.href = cameFromBot;
  }

  render() {
    let backLink = null;

    if (cameFromBot !== null) {
      backLink = <Button variant="warning" size="sm" onClick={() => this.backToBot()} title="Go back to your local OxidizeBot instance">
        Back to <img src={logo} width="18" height="18"></img>
      </Button>;
    }

    let button = (
      <Form inline key="second">
        <ButtonGroup>
          {backLink}
          <Button size="sm" onClick={this.login.bind(this)} title="Sign in through Twitch">
            <b>Sign in with</b>&nbsp;<img src={twitchLogo} height="16px" width="48px" alt="twitch" />
          </Button>
        </ButtonGroup>
      </Form>
    );

    if (currentUser) {
      button = (
        <Dropdown key="second">
          <ButtonGroup>
            {backLink}
            <Dropdown.Toggle size="sm">Signed in: <b>{currentUser.login}</b></Dropdown.Toggle>
          </ButtonGroup>

          <Dropdown.Menu>
            <Dropdown.Item as={Link} to="/connections">
              My Connections
            </Dropdown.Item>
            <Dropdown.Divider />
            <Dropdown.Item onClick={this.logout.bind(this)}>
              Sign out <FontAwesomeIcon icon="sign-out-alt" />
            </Dropdown.Item>
          </Dropdown.Menu>
        </Dropdown>
      );
    }

    return button;
  }
}