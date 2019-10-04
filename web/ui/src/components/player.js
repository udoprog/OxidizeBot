import React from "react";
import { Row, Col, Table, Alert } from "react-bootstrap";
import { api } from "../globals.js";
import { RouteLayout } from "./Layout.js";
import Loading from "./Loading.js";
import If from "./If.js";

export default class Player extends React.Component {
  constructor(props) {
    super(props);
    this.state = {
      error: null,
      loading: true,
      player: null,
    };
  }

  async componentDidMount() {
    await this.refresh();
  }

  async refresh() {
    try {
      let player = await api.player(this.props.match.params.id);
      this.setState({player, loading: false});
    } catch(e) {
      this.setState({error: e, loading: false});
    }
  }

  render() {
    let content = null;

    if (!this.state.loading) {
      if (this.state.error !== null) {
        content = <Alert variant="danger" className="center">{this.state.error}</Alert>;
      } else if (this.state.player === null) {
        content = <Alert variant="warning" className="center">User doesn't have an active player!</Alert>;
      } else {
        content = <>
          <Table>
            <thead>
              <tr>
                <th></th>
                <th scope="col">Song</th>
                <th scope="col">Artist</th>
                <th scope="col">Length</th>
                <th scope="col">Requested By</th>
              </tr>
            </thead>
            <tbody>
            {this.state.player.items.map(({name, track_url, artists, duration, user}, index) => {
              let classes = "";
              let current = index;

              if (index == 0) {
                current = <span title="Current Song">&#9654;</span>;
                classes = "current";
              }

              let userInfo = null;

              if (user !== null) {
                userInfo = <a href={`https://twitch.tv/${user}`}>{user}</a>;
              } else {
                userInfo = <a href="https://awoiaf.westeros.org/index.php/Faceless_Men"><em>No One</em></a>;
              }

              return (
                <tr key={index} className={classes}>
                  <th>{current}</th>
                  <td>
                    <a href={track_url}>{name}</a>
                  </td>
                  <td>{artists}</td>
                  <td>{duration}</td>
                  <td>{userInfo}</td>
                </tr>
              );
            })}
            </tbody>
          </Table>
        </>;
      }
    }

    return (
      <RouteLayout>
        <h2 className="page-title">Player for {this.props.match.params.id}</h2>
        <Loading isLoading={this.state.loading} />
        {content}
      </RouteLayout>
    );
  }
}