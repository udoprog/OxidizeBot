import {Spinner} from "../utils.js";
import React from "react";
import {Alert, Table} from "react-bootstrap";

export default class Authentication extends React.Component {
  constructor(props) {
    super(props);
    this.api = this.props.api;

    this.state = {
      loading: false,
      error: null,
      auth: [],
    };
  }

  componentWillMount() {
    this.setState({
      loading: true,
    });

    this.api.auth()
      .then(auth => {
        this.setState({
          loading: false,
          error: null,
          auth,
        });
      },
      e => {
        this.setState({
          loading: false,
          error: `failed to request needed authentication: ${e}`,
        });
      });
  }

  render() {
    let error = null;

    if (this.state.error) {
      error = <Alert variant="warning">{this.state.error}</Alert>;
    }

    let needsAuth = null;
    let content = null;

    if (this.state.loading) {
      content = <Spinner />;
    } else {
      if (this.state.auth.length == 0) {
        content = (
          <Alert variant="success">
            Everything is successfully authenticated!
          </Alert>
        );
      } else {
        needsAuth = (
          <Alert variant="danger">
            You have things that require authentication. Click below to authenticate.
          </Alert>
        );

        content = (
          <Table responsive="sm">
            <tbody>
              {this.state.auth.map((a, id) => {
                return (
                  <tr key={id}>
                    <td><a href={a.url}>{a.title}</a></td>
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
        <h4>Authentication</h4>
        {error}
        {needsAuth}
        {content}
      </div>
    );
  }
}