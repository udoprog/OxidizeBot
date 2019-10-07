import React from "react";
import {Form, Button, ButtonGroup, Row, Col, InputGroup} from "react-bootstrap";
import {FontAwesomeIcon} from "@fortawesome/react-fontawesome";
import * as ReactMarkdown from 'react-markdown';

const SECRET_PREFIX = "secrets/";

function confirmButtons({what, onConfirm, onCancel, confirmDisabled}) {
  confirmDisabled = confirmDisabled || false;

  return [
    <Button key="cancel" title={`Cancel ${what}`} variant="primary" size="sm" onClick={e => onCancel(e)}>
      <FontAwesomeIcon icon="window-close" />
    </Button>,
    <Button key="confirm" title={`Confirm ${what}`} disabled={confirmDisabled} variant="danger" size="sm" onClick={e => onConfirm(e)}>
      <FontAwesomeIcon icon="check-circle" />
    </Button>
  ];
}

export default class Setting extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      delete: false,
      edit: null,
      secretShown: false,
      editValue: null,
      hideCountdown: 0,
      hideInterval: null,
    };
  }

  /**
   * Delete the given setting.
   *
   * @param {string} key key of the setting to delete.
   */
  delete(key) {
    this.props.onDelete(key);
  }

  /**
   * Edit the given setting.
   *
   * @param {string} key key of the setting to edit.
   * @param {string} value the new value to edit it to.
   */
  edit(key, control, value) {
    this.props.onEdit(key, control, value);
  }

  render() {
    let setting = this.props.setting;
    let keyOverride = this.props.keyOverride;

    let isSecretShown = this.state.secretShown;

    // onChange handler used for things which support immediate editing.
    let renderOnChange = value => {
      this.edit(setting.key, setting.control, value);
    };

    let buttons = [];
    let isSecret = setting.key.startsWith(SECRET_PREFIX) || setting.secret;
    let isNotSet = setting.value === null;

    if (isSecretShown) {
      let hide = () => {
        if (this.state.hideInterval !== null) {
          clearInterval(this.state.hideInterval);
        }

        this.setState({
          secretShown: false,
          hideInterval: null,
        });
      };

      buttons.push(
        <Button title="Hide the secret value" key="show" size="sm" variant="secondary" className="action" disabled={this.state.loading} onClick={hide}>
          <FontAwesomeIcon icon="eye-slash" />
          <span className="settings-countdown">
            {this.state.hideCountdown}s
          </span>
        </Button>
      );
    }

    if (isSecret && !isSecretShown && !isNotSet) {
      let hideFeedback = () => {
        this.setState(state => {
          if (state.hideCountdown <= 1) {
            clearInterval(state.hideInterval);
            return {secretShown: false, hideCountdown: 0};
          }

          return {hideCountdown: state.hideCountdown - 1};
        });
      };

      let show = () => {
        this.setState({
          hideCountdown: 10,
          secretShown: true,
          hideInterval: setInterval(hideFeedback, 1000),
        });
      };

      buttons.push(
        <Button title="Show the secret value" key="show" size="sm" variant="secondary" className="action" disabled={this.state.loading} onClick={show}>
          <FontAwesomeIcon icon="eye" />
        </Button>
      );
    }

    if (setting.control.optional && !isSecretShown) {
      let del = () => {
        this.setState({
          delete: true,
        });
      };

      if (setting.value !== null) {
        buttons.push(
          <Button key="delete" size="sm" variant="danger" className="action" disabled={this.state.loading} onClick={del}>
            <FontAwesomeIcon icon="trash" />
          </Button>
        );
      }
    }

    if (setting.control.hasEditControl()) {
      if (!isSecretShown) {
        let edit = () => {
          let value = setting.value;

          if (value == null) {
            value = setting.control.default();
          }

          let edit = setting.control.editControl();
          let editValue = setting.control.edit(value);

          this.setState({
            edit,
            editValue,
          });
        };

        buttons.push(
          <Button key="edit" size="sm" variant="info" className="action" disabled={this.state.loading} onClick={edit}>
            <FontAwesomeIcon icon="edit" />
          </Button>
        );
      }
    }

    let value = null;

    if (isNotSet) {
      value = <em title="Value not set">not set</em>;;
    } else {
      if (isSecret && !isSecretShown) {
        value = <b title="Secret value, only showed when editing">****</b>;
      } else {
        value = setting.control.render(setting.value, renderOnChange);
      }
    }

    if (this.state.delete) {
      buttons = confirmButtons({
        what: "deletion",
        onConfirm: () => {
          this.setState({
            delete: false,
          });

          this.delete(setting.key);
        },
        onCancel: () => {
          this.setState({
            delete: false,
          })
        },
      });
    }

    if (this.state.edit !== null) {
      let isValid = this.state.edit.validate(this.state.editValue);

      let save = (e) => {
        e.preventDefault();

        if (isValid) {
          let value = this.state.edit.save(this.state.editValue);
          this.edit(setting.key, setting.control, value);
          this.setState({edit: null});
        }

        return false;
      };

      let control = this.state.edit.render(this.state.editValue, editValue => {
        this.setState({editValue});
      }, isValid);

      value = (
        <Form onSubmit={e => save(e)}>
          <InputGroup size="sm">{control}</InputGroup>
        </Form>
      );

      buttons = confirmButtons({
        what: "edit",
        confirmDisabled: !isValid,
        onConfirm: e => save(e),
        onCancel: () => {
          this.setState({
            edit: null,
          })
        },
      });
    }

    if (buttons.length > 0) {
      buttons = (
        <div className="ml-3">
          <ButtonGroup>{buttons}</ButtonGroup>
        </div>
      );
    }

    let key = keyOverride || setting.key;

    if (this.props.useTitle && !!setting.title) {
      key = <ReactMarkdown source={setting.title} />
    }

    let doc = null;

    if (!this.props.disableDoc) {
      doc = (
        <div className="settings-key-doc">
          <ReactMarkdown source={setting.doc} />
        </div>
      );
    }

    return (
      <tr>
        <td>
          <Row>
            <Col lg="4" className="settings-key mb-1">
              <div className="settings-key-name mb-1">{key}</div>
              {doc}
            </Col>

            <Col lg="8">
              <div className="d-flex align-items-top">
                <div className="flex-fill align-middle">{value}</div>
                {buttons}
              </div>
            </Col>
          </Row>
        </td>
      </tr>
    );
  }
}