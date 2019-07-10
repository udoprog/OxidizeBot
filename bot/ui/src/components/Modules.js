import React from "react";
import {Nav, Row, Col} from "react-bootstrap";
import {Route, Link} from "react-router-dom";
import ConfigurationPrompt from "./ConfigurationPrompt";

function Player(props) {
  return (
    <div>
      <h3>Music Player</h3>

      <p>
        Handles playing music in SetMod.
      </p>

      <ConfigurationPrompt
        group={true}
        filterable={true}
        filter={{prefix: ["player", "song"]}}
        {...props}
        />
    </div>
  );
}

function Gtav(props) {
  return (
    <div>
      <h3>ChaosMod</h3>

      <p>
        <a href="https://github.com/udoprog/ChaosMod">ChaosMod</a> is a mod for GTA V that allows viewers to interact with your game.
      </p>

      <ConfigurationPrompt
        group={true}
        filterable={true}
        filter={{prefix: ["gtav"]}}
        {...props}
        />
    </div>
  );
}

function Currency(props) {
  return (
    <div>
      <h3>Stream Currency</h3>

      <p>
        A stream currency is a kind of loyalty points system.
      </p>

      <p>
        SetMod has a sophisticated system that integrated with many parts of the bot, and different database backends.
      </p>

      <ConfigurationPrompt
        group={true}
        filterable={true}
        filter={{prefix: ["currency"]}}
        {...props}
        />
    </div>
  );
}

function ChatLog(props) {
  return (
    <div>
      <h3>Chat Log</h3>

      <p>
        Experimental Chat Log Support
      </p>

      <ConfigurationPrompt
        group={true}
        filter={{prefix: ["chat-log"]}}
        {...props}
        />
    </div>
  );
}

function Index(props) {
  return (
    <div>
      <p>
        This section contains a list of all features that can be toggled on or off.
        Each feature might have more settings. If so, they are detailed to the left.
      </p>

      <ConfigurationPrompt
        useTitle={true}
        filterable={true}
        filter={{feature: true}}
        {...props} />
    </div>
  )
}

export default class Modules extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    let path = this.props.location.pathname;

    return (
      <Row>
        <Col sm="2">
          <Nav className="flex-column" variant="pills">
            <Nav.Link as={Link} active={path === "/modules/player"} to="/modules/player">
              Music Player
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/modules/currency"} to="/modules/currency">
              Stream Currency
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/modules/chat-log"} to="/modules/chat-log">
              Chat Log
            </Nav.Link>
            <Nav.Link as={Link} active={path === "/modules/gtav"} to="/modules/gtav">
              ChaosMod
            </Nav.Link>
          </Nav>
        </Col>
        <Col>
          <Route path="/modules" exact render={props => <Index api={this.props.api} {...props} />} />
          <Route path="/modules/player" render={props => <Player api={this.props.api} {...props} />} />
          <Route path="/modules/currency" render={props => <Currency api={this.props.api} {...props} />} />
          <Route path="/modules/chat-log" render={props => <ChatLog api={this.props.api} {...props} />} />
          <Route path="/modules/gtav" render={props => <Gtav api={this.props.api} {...props} />} />
        </Col>
      </Row>
    );
  }
}