export class Regex {
  constructor(pattern) {
    this.pattern = new RegExp(pattern);
  }

  validate(value) {
    return this.pattern.test(value);
  }
}

export class None {
  constructor() {
  }

  validate(value) {
    return true;
  }
}

export function decode(format) {
  switch (format.type) {
    case "regex":
      return new Regex(format.pattern);
    default:
      return new None();
  }
}