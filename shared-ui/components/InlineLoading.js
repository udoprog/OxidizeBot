import React from "react";

export default function Loading(props) {
  if (props.isLoading !== undefined && !props.isLoading) {
    return null;
  }

  return <span className="oxi-inline-loading spinner-border" role="status">
    <span className="sr-only">Loading...</span>
  </span>;
}