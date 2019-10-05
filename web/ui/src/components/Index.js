import React from "react";
import { Card, CardDeck } from "react-bootstrap";
import { RouteLayout } from "./Layout.js";
import dollarImg from "../assets/dollar.png";
import toolboxImg from "../assets/toolbox.png";
import cloudImg from "../assets/cloud.png";
import twitchDarkLogo from "../assets/twitch-dark.png";
import windowsImg from "../assets/windows.svg";
import debianImg from "../assets/debian.svg";
import macImg from "../assets/mac.svg";
import SVG from 'react-inlinesvg';

export default class Index extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <RouteLayout>
        <h2 className="page-title">OxidizeBot</h2>

        <div className="center mb-4">
          <b>OxidizeBot</b> is the high octane <a href="https://twitch.tv"><img src={twitchDarkLogo} height="16px" width="48px" alt="twitch" /></a> bot written in <a href="https://rust-lang.org">Rust</a>!
        </div>

        <CardDeck className="mb-4">
          <Card>
            <Card.Img variant="top" src={dollarImg} />
            <Card.Body>
              <Card.Title className="center"><b>Free</b> and <b>Open Source</b></Card.Title>
              <Card.Text>
                OxidizeBot doesn't cost you anything,
                and its source code is available on <a href="https://github.com/udoprog/OxidizeBot">GitHub</a> for anyone to tinker with!
              </Card.Text>
            </Card.Body>
          </Card>
          <Card>
            <Card.Img variant="top" src={toolboxImg} />
            <Card.Body>
              <Card.Title className="center"><b>Packed</b> with <b>Features</b></Card.Title>
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
              <Card.Title className="center">Runs on <b>Your Computer</b></Card.Title>
              <Card.Text>
                <em>You</em> own your data.
                It uses <em>your</em> internet for the best possible latency.
                It's light on system resources (Low CPU and about 50MB of ram).
                And running locally means it can perform rich interactions with your games like <a href="https://github.com/udoprog/ChaosMod">Chaos%</a>.
              </Card.Text>
            </Card.Body>
          </Card>
        </CardDeck>

        <h4 className="center mb-4">Download</h4>

        <CardDeck>
          <Card bg="light">
            <Card.Img as={SVG} src={windowsImg} height="80px" className="mb-3 mt-3" />
            <Card.Body>
              <Card.Title className="center">Windows</Card.Title>
              <Card.Text className="center">
                <a href="https://github.com/udoprog/OxidizeBot/releases/download/1.0.0-beta.19/oxidize-1.0.19-x86_64.msi">1.0.0-beta.19 Beta Installer (.msi)</a>
              </Card.Text>
            </Card.Body>
          </Card>
          <Card bg="light">
            <Card.Img as={SVG} src={debianImg} height="80px" className="mb-3 mt-3" />
            <Card.Body>
              <Card.Title className="center">Debian</Card.Title>
              <Card.Text className="center">
                <a href="https://github.com/udoprog/OxidizeBot/releases/download/1.0.0-beta.19/oxidize_1.0.0.beta.19_amd64.deb">1.0.0-beta.19 Beta Installer (.deb)</a>
              </Card.Text>
            </Card.Body>
          </Card>
        </CardDeck>
      </RouteLayout>
    );
  }
}