import React from "react";

export default function Loading(props) {
  if (!props.error) {
    return null;
  }

  return (
    <div className="oxi-error alert alert-danger">{props.error.toString()}</div>
  );
}