import React from "react";
import { Row, Col, Table, Alert } from "react-bootstrap";
import { api } from "../globals.js";
import { ApiError } from "../api.js";
import { RouteLayout } from "./Layout.js";
import Loading from 'shared-ui/components/Loading';
import * as enableRemoteUpdates from '../assets/enable-remote-updates.png';

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
      this.setState({error: null, player, loading: false});
    } catch(error) {
      this.setState({error, player: null, loading: false});
    }
  }

  /**
   * Render the relevant error as an Alert.
   */
  renderError(error) {
    if (error instanceof ApiError) {
      if (error.notFound()) {
        return <Alert variant="danger" className="oxi-center">
          <b>Player not found.</b>

          <div className="player-not-found-hint">
            Do you expect to see something here?<br />
            Maybe you forgot to <a href="http://localhost:12345/settings?q=%5Eremote%2F">enable remote updates</a> in your bot local settings:
          </div>
          
          <div className="player-not-found-hint-image">
            <img src={enableRemoteUpdates} />
          </div>
        </Alert>;
      }
    }

    return <Alert variant="danger" className="oxi-center">{error.toString()}</Alert>;
  }

  render() {
    let content = null;

    if (!this.state.loading) {
      if (this.state.error !== null) {
        content = this.renderError(this.state.error);
      } else if (this.state.player === null) {
        content = <Alert variant="warning" className="oxi-center">User doesn't have an active player!</Alert>;
      } else {
        content = <>
          <Table className="player" striped bordered hover>
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
                classes = "oxi-current";
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
        <h2 className="oxi-page-title">Playlist for {this.props.match.params.id}</h2>
        <Loading isLoading={this.state.loading} />
        {content}
      </RouteLayout>
    );
  }
}