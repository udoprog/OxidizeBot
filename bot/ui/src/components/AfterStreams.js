import {Spinner} from "../utils.js";
import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";

export default class AfterStreams extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      data: null,
    };
  }

  componentWillMount() {
    if (this.state.loading) {
      return;
    }

    this.list()
  }

  /**
   * Refresh the list of after streams.
   */
  list() {
    this.setState({
      loading: true,
    });

    this.api.afterStreams()
      .then(data => {
        this.setState({
          loading: false,
          error: null,
          data,
        });
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to request after streams: ${e}`,
          data: null,
        });
      });
  }

  /**
   * Delete the given afterstream.
   *
   * @param {number} id afterstream id to delete
   */
  delete(id) {
    this.api.deleteAfterStream(id)
      .then(() => {
        return this.list();
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to delete after stream: ${e}`,
        });
      });
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let refresh = null;
    let loading = null;

    if (this.state.loading) {
      loading = <Spinner />;
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh right" />;
    } else {
      refresh = <FontAwesomeIcon icon="sync" className="title-refresh clickable right" onClick={() => this.list()} />;
    }

    let content = null;

    if (this.state.data) {
      if (this.state.data.length === 0) {
        content = (
          <Alert variant="info">
            No After Streams!
          </Alert>
        );
      } else {
        content = (
          <Table responsive="sm">
            <thead>
              <tr>
                <th>User</th>
                <th className="table-fill">Message</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {this.state.data.map((a, id) => {
                return (
                  <tr key={id}>
                    <td className="afterstream-user">
                      <a className="afterstream-name" href={`https://twitch.tv/${a.user}`}>@{a.user}</a>
                      <span className="afterstream-added-at">
                        <span className="afterstream-at">at</span>
                        <span className="afterstream-datetime datetime">{a.added_at}</span>
                      </span>
                    </td>
                    <td><code>{a.text}</code></td>
                    <td>
                      <Button size="sm" variant="danger" className="action" onClick={() => this.delete(a.id)}>
                        <FontAwesomeIcon icon="trash" />
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </Table>
        );
      }
    }

    return (
      <div>
        <h2>
          After Streams
          {refresh}
        </h2>
        {error}
        {content}
        {loading}
      </div>
    );
  }
}