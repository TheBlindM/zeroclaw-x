type ClassName = string | false | null | undefined;

export function cn(...classes: ClassName[]) {
  return classes.filter(Boolean).join(" ");
}
