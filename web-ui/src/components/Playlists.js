import React from "react";
import { Table, Alert, Spinner } from "react-bootstrap";
import { Link } from "react-router-dom";
import { api } from "../globals.js";
import { RouteLayout } from "./Layout";
import Loading from "./Loading";

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
          <Alert variant="danger" className="center">{this.state.error.toString()}</Alert>
        );
      } else if (this.state.players.length === 0) {
        content = (
          <Alert variant="warning" className="center">
            No active players!
          </Alert>
        );
      } else {
        content = (
          <Table bordered hover>
            <tbody>
              {this.state.players.map(p => {
                return (
                  <tr key={p}>
                    <td><Link to={`/player/${p}`}>{p}</Link></td>
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
        <h2 className="page-title">Playlists</h2>

        <p className="center">
          This page features people who have enabled remote playlists in OxidizeBot.
        </p>

        <Loading isLoading={this.state.loading} />
        {content}
      </RouteLayout>
    );
  }
}