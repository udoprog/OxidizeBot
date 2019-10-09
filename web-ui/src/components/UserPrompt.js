import React from "react";
import { Form, Button, Alert } from "react-bootstrap";
import { api } from "../globals.js";
import twitchLogo from "../assets/twitch.png";

export default class UserPrompt extends React.Component {
  constructor(props) {
    super(props);
  }

  async login() {
    let result = await api.authLogin();
    location.href = result.auth_url;
  }

  render() {
    return (
      <>
        <Alert variant="warning" className="oxi-center">
          <div className="mb-3">
            This page requires you to sign in!
          </div>

          <Form>
            <Button size="xl" onClick={this.login.bind(this)} title="Sign in through Twitch">
              Sign in with <img src={twitchLogo} height="16px" width="48px" alt="twitch" />
            </Button>
          </Form>
        </Alert>
      </>
    );
  }
}