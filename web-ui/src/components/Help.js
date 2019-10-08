import React from "react";
import { RouteLayout } from "./Layout.js";
import commands from 'toml-loader!../../../shared/commands.toml';
import Help from 'shared-ui/components/Help';

export default class HelpPage extends React.Component {
  constructor(props) {
    super(props);
  }

  render() {
    return (
      <RouteLayout>
        <Help commands={commands} {...this.props} />
      </RouteLayout>
    );
  }
}