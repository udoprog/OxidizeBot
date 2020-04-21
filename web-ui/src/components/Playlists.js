import React from "react";
import { Table, Alert, Spinner } from "react-bootstrap";
import { Link } from "react-router-dom";
import { api } from "../globals.js";
import { RouteLayout } from "./Layout";
import Loading from 'shared-ui/components/Loading';
import * as utils from "../utils";

export default class Players extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      loading: true,
      players: [],
      error: null,
    };
  }

  async componentDidMount() {
    try {
      let players = await api.players();
      this.setState({players, loading: false});
    } catch(e) {
      this.setState({error: e, loading: false});
    }
  }

  render() {
    let content = null;

    if (!this.state.loading) {
      if (this.state.error !== null) {
        content = (
          <Alert variant="danger" className="oxi-center">{this.state.error.toString()}</Alert>
        );
      } else if (this.state.players.length === 0) {
        content = (
          <Alert variant="warning" className="oxi-center">
            No active players!
          </Alert>
        );
      } else {
        content = (
          <Table className="playlists" striped bordered hover>
            <thead>
              <tr>
                <th>User</th>
                <th width="1%">Last&nbsp;Update</th>
              </tr>
            </thead>
            <tbody>
              {this.state.players.map(p => {
                let lastUpdate = "?";

                if (!!p.last_update) {
                  lastUpdate = utils.humanDurationSince(new Date(p.last_update));
                }

                return (
                  <tr key={p.user_login}>
                    <td><Link alt="Go to player" to={`/player/${p.user_login}`}>{p.user_login}</Link></td>
                    <td className="playlists-last-update">{lastUpdate}</td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return (
      <RouteLayout>
        <h2 className="oxi-page-title">Playlists</h2>

        <p className="oxi-center">
          This page features people who have enabled remote playlists in OxidizeBot.
        </p>

        <Loading isLoading={this.state.loading} />
        {content}
      </RouteLayout>
    );
  }
}