export class Base {
  constructor(optional) {
    this.optional = optional;
  }

  edit() {
    throw new Error("missing edit() implementation");
  }

  hasEditControl() {
    return true;
  }

  isSingular() {
    return true;
  }
}