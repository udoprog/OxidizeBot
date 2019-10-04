import React from "react";
import { Card, CardDeck } from "react-bootstrap";
import { RouteLayout } from "./Layout.js";
import dollarImg from "../assets/dollar.png";
import toolboxImg from "../assets/toolbox.png";
import cloudImg from "../assets/cloud.png";
import twitchDarkLogo from "../assets/twitch-dark.png";

export default class Index extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <RouteLayout>
        <h2 className="page-title mb-3">OxidizeBot</h2>

        <Card bg="light" className="mb-4">
          <Card.Body>
            <Card.Text>
              <b>OxidizeBot</b> is the high octane <a href="https://twitch.tv"><img src={twitchDarkLogo} height="16px" width="48px" alt="twitch" /></a> bot written in <a href="https://rust-lang.org">Rust</a>!
            </Card.Text>
          </Card.Body>
        </Card>

        <CardDeck>
          <Card>
            <Card.Img variant="top" src={dollarImg} />
            <Card.Body>
              <Card.Title><b>Free</b> and <b>Open Source</b></Card.Title>
              <Card.Text>
                OxidizeBot doesn't cost you anything,
                and its source code is available on <a href="https://github.com/udoprog/OxidizeBot">GitHub</a> for anyone to tinker with!
              </Card.Text>
            </Card.Body>
          </Card>
          <Card>
            <Card.Img variant="top" src={toolboxImg} />
            <Card.Body>
              <Card.Title><b>Packed</b> with features</Card.Title>
              <Card.Text>
                Plays music, moderates your chat, plays games, you name it!
              </Card.Text>
              <Card.Text>
                If you feel something is missing, feel free to <a href="https://github.com/udoprog/OxidizeBot/issues">open an issue</a>.
              </Card.Text>
            </Card.Body>
          </Card>
          <Card>
            <Card.Img variant="top" src={cloudImg} />
            <Card.Body>
              <Card.Title>Runs on your <b>Computer</b></Card.Title>
              <Card.Text>
                You own your data.
                It uses your network connection for the best possible latency.
                It's light on system resources (Low CPU and about 50MB of ram).
                And running locally means it can perform rich interactions with your games like <a href="https://github.com/udoprog/ChaosMod">Chaos%</a>.
              </Card.Text>
            </Card.Body>
          </Card>
        </CardDeck>
      </RouteLayout>
    );
  }
}