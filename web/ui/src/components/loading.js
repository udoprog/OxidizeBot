import React from "react";
import {Spinner} from "react-bootstrap";

export default function Loading(props) {
  if (!props.isLoading) {
    return null;
  }

  return (
    <div className="loading">
      <Spinner animation="border" role="status">
        <span className="sr-only">Loading...</span>
      </Spinner>
    </div>
  );
}