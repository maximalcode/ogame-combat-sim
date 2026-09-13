import { useId } from "react";
import { numericInput } from "./model";
interface Props {
  label: string;
  value: string;
  max: number;
  integer?: boolean;
  onChange: (value: string) => void;
}
export function NumberField({ label, value, max, integer = true, onChange }: Readonly<Props>) {
  const hintId = useId();
  return (
    <label className="number-field">
      <span>{label}</span>
      <input
        type="number"
        aria-label={label}
        aria-describedby={value === "" ? hintId : undefined}
        min="0"
        max={max}
        step={integer ? "1" : "any"}
        value={value}
        className={value === "" ? "missing" : ""}
        onChange={(event) => {
          onChange(numericInput(event.target.value, max, integer));
        }}
      />
      {value === "" && <small id={hintId}>Assumed zero · angenommen: 0</small>}
    </label>
  );
}
