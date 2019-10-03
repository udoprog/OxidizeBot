import React from "react";
import { Navbar, NavDropdown, Form, Button, Dropdown } from "react-bootstrap";
import { Link } from "react-router-dom";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { api, currentUser } from "../globals.js";
import twitchLogo from "../assets/twitch.png";

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

  render() {
    let button = (
      <Form inline key="second">
        <Button size="sm" onClick={this.login.bind(this)} title="Sign in through Twitch">
          <b>Sign in with</b>&nbsp;<img src={twitchLogo} height="16px" width="48px" alt="twitch" />
        </Button>
      </Form>
    );

    if (currentUser) {
      button = (
        <Dropdown key="second">
          <Dropdown.Toggle size="sm">Signed in: <b>{currentUser.user}</b></Dropdown.Toggle>

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

    return [
      button
    ];
  }
}