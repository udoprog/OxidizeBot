import React from "react";
import {Button, Alert, Table} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import ConfigurationPrompt from "./ConfigurationPrompt";
import {Loading, Error} from 'shared-ui/components';

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

  async componentDidMount() {
    await this.list();
  }

  /**
   * Refresh the list of after streams.
   */
  async list() {
    this.setState({
      loading: true,
    });

    try {
      let data = await this.api.afterStreams();

      this.setState({
        loading: false,
        error: null,
        data,
      });
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to request after streams: ${e}`,
        data: null,
      });
    }
  }

  /**
   * Delete the given afterstream.
   *
   * @param {number} id afterstream id to delete
   */
  async delete(id) {
    try {
      await this.api.deleteAfterStream(id);
      await this.list();
    } catch(e) {
      this.setState({
        loading: false,
        error: `failed to delete after stream: ${e}`,
      });
    }
  }

  render() {
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

    return <>
      <h1 className='oxi-page-title'>After Streams</h1>
      <Loading isLoading={this.state.loading} />
      <Error error={this.state.error} />
      <ConfigurationPrompt api={this.api} filter={{prefix: ["afterstream"]}} />
      {content}
    </>;
  }
}