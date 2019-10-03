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
      player: null,
    };
  }

  async componentDidMount() {
    let player = await api.player(this.props.match.params.id);
    this.setState({player});
  }

  render() {
    let content = null;

    if (!this.state.loading) {
      if (this.state.player !== null) {
        content = <>
          <h2 className="page-title">Player for {this.props.match.params.id}</h2>

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
      } else {
        content = <Alert variant="danger">No player for {this.props.match.params.id}</Alert>;
      }
    }

    return (
      <RouteLayout>
        <Loading isLoading={this.state.loading} />
        {content}
      </RouteLayout>
    );
  }
}