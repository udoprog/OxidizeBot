import React from "react";
import { Card, CardDeck } from "react-bootstrap";
import { RouteLayout } from "./layout.js";
import dollarImg from "../assets/dollar.png";
import toolboxImg from "../assets/toolbox.png";
import cloudImg from "../assets/cloud.png";

export default class Index extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <RouteLayout>
        <Card bg="light" className="mb-4">
          <Card.Header>Welcome to <b>setbac.tv</b>, the home of OxidizeBot!</Card.Header>
          <Card.Body>
            <Card.Text>
              OxidizeBot is the <b>Free</b> and <b>Open Source</b> Twitch Bot written in Rust!
            </Card.Text>
          </Card.Body>
        </Card>

        <CardDeck>
          <Card>
            <Card.Img variant="top" src={dollarImg} />
            <Card.Body>
              <Card.Title><b>Free</b> and <b>Open Source</b></Card.Title>
              <Card.Text>
                OxidizeBot doesn't cost you anything!
                And it's source code is available <a href="https://github.com/udoprog/OxidizeBot">on GitHub</a>!
              </Card.Text>
            </Card.Body>
          </Card>
          <Card>
            <Card.Img variant="top" src={toolboxImg} />
            <Card.Body>
              <Card.Title>Packed with features</Card.Title>
              <Card.Text>
                Plays music, moderates your chat, has a rich authentication system.<br />
                If you feel something is missing, <a href="https://github.com/udoprog/OxidizeBot">open an issue</a>.
              </Card.Text>
            </Card.Body>
          </Card>
          <Card>
            <Card.Img variant="top" src={cloudImg} />
            <Card.Body>
              <Card.Title>Runs on your computer</Card.Title>
              <Card.Text>
                The bot has the same latency as you do.
                It can interact with your desktop and automate tedious task.
              </Card.Text>
            </Card.Body>
          </Card>
        </CardDeck>
      </RouteLayout>
    );
  }
}