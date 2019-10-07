import React from "react";
import {Spinner} from "react-bootstrap";

export default function If(props) {
  let visible = false;

  if (props.isNot !== undefined && props.isNot) {
    return null;
  }

  if (props.is !== undefined && !props.is) {
    return null;
  }

  return props.children;
}