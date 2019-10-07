import React from "react";
import { RouteLayout } from "./Layout.js";
import { Alert, Table, Button, Form, FormControl, InputGroup, ButtonGroup } from "react-bootstrap";
import { api, currentConnections, currentUser } from "../globals.js";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import copy from 'copy-to-clipboard';
import Loading from "./Loading.js";
import If from "./If.js";
import UserPrompt from "./UserPrompt";
import Connection from "./Connection";
import { CommandGroup } from "shared-ui";
import commands from 'json-loader!yaml-loader!../../../shared/commands.yaml';

export default class Help extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      loading: true,
      error: null,
      groups: commands.groups,
    }
  }

  componentDidMount() {
    this.setState({loading: false});
  }

  render() {
    let error = null;

    return (
      <RouteLayout>
        <h2 className="page-title">Help</h2>

        <Loading isLoading={this.state.loading} />
        {error}

        {this.state.groups.map((c, i) => {
          return <CommandGroup key={i} {...c} />;
        })}
      </RouteLayout>
    );
  }
}