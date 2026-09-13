import { useState } from "react";
import { availableTypes, unitName } from "./catalog";
import {
  blankNumbers,
  MAX_COUNT,
  restoreSelection,
  scaleSelection,
  selectQuantity,
  STATS,
  type Fleet,
  type Unit,
} from "./model";
import { NumberField } from "./NumberField";

interface Props {
  attacker: boolean;
  fleet: Fleet;
  onChange: (fleet: Fleet) => void;
}
function UnitPicker({ attacker, fleet, onChange }: Readonly<Props>) {
  const [id, setId] = useState("");
  const [count, setCount] = useState("");
  const choices = availableTypes(attacker).filter(
    (unit) => !fleet.units.some((existing) => existing.id === unit.id),
  );
  const updateStock = (id: string, available: string) => {
    onChange({
      ...fleet,
      units: fleet.units.map((unit) =>
        unit.id === id ? selectQuantity({ ...unit, available }, unit.selected) : unit,
      ),
    });
  };
  return (
    <details className="unit-picker">
      <summary>{attacker ? "Bestand manuell ergänzen" : "Gegner manuell ergänzen"}</summary>
      <div className="picker-fields">
        <label>
          Einheit
          <select
            aria-label="Einheit"
            value={id}
            onChange={(event) => {
              setId(event.target.value);
            }}
          >
            <option value="">Einheit wählen</option>
            {choices.map((unit) => (
              <option key={unit.id} value={unit.id}>
                {unitName(unit.id)}
              </option>
            ))}
          </select>
        </label>
        <NumberField label="Anzahl hinzufügen" value={count} max={MAX_COUNT} onChange={setCount} />
        <button
          type="button"
          disabled={!choices.some((unit) => unit.id === id)}
          onClick={() => {
            onChange({
              ...fleet,
              units: [
                ...fleet.units,
                { id, available: count, selected: count, lifeform: blankNumbers() },
              ],
            });
            setId("");
            setCount("");
          }}
        >
          Hinzufügen
        </button>
      </div>
      {attacker && fleet.units.length > 0 && (
        <div className="stock-editor">
          <p>Snapshot korrigieren (begrenzt auch deine Auswahl):</p>
          {fleet.units.map((unit) => (
            <NumberField
              key={unit.id}
              label={`Bestand ${unitName(unit.id)}`}
              value={unit.available}
              max={MAX_COUNT}
              onChange={(value) => {
                updateStock(unit.id, value);
              }}
            />
          ))}
        </div>
      )}
    </details>
  );
}
function UnitRow({
  unit,
  attacker,
  change,
}: {
  readonly unit: Unit;
  readonly attacker: boolean;
  readonly change: (unit: Unit) => void;
}) {
  const name = unitName(unit.id);
  const maximum = attacker ? Number(unit.available) : MAX_COUNT;
  const changeQuantity = (value: string) => {
    change(selectQuantity(unit, value, maximum));
  };
  return (
    <tr>
      <th scope="row">
        <span className="unit-mark">
          <img
            src="/art/fleet.svg"
            alt=""
            onError={(event) => {
              event.currentTarget.style.visibility = "hidden";
            }}
          />
          <small>{unit.id}</small>
        </span>
        {name}
      </th>
      {attacker && (
        <td>
          <span className={unit.available === "" ? "assumption" : ""}>
            {unit.available === ""
              ? "Assumed zero"
              : Number(unit.available).toLocaleString("de-DE")}
          </span>
        </td>
      )}
      <td>
        <div className="quantity">
          <button
            type="button"
            aria-label={`${name} weniger`}
            disabled={unit.selected === ""}
            onClick={() => {
              changeQuantity(String(Number(unit.selected) - 1));
            }}
          >
            −
          </button>
          <NumberField
            label={`${name} Menge`}
            value={unit.selected}
            max={maximum}
            onChange={changeQuantity}
          />
          <button
            type="button"
            aria-label={`${name} mehr`}
            disabled={unit.selected === ""}
            onClick={() => {
              changeQuantity(String(Number(unit.selected) + 1));
            }}
          >
            +
          </button>
        </div>
      </td>
    </tr>
  );
}
function Modifiers({ fleet, onChange }: Pick<Readonly<Props>, "fleet" | "onChange">) {
  const changeLifeform = (unit: Unit, key: keyof Unit["lifeform"], value: string) => {
    onChange({
      ...fleet,
      units: fleet.units.map((current) =>
        current.id === unit.id
          ? { ...current, lifeform: { ...current.lifeform, [key]: value } }
          : current,
      ),
    });
  };
  return (
    <details className="modifiers">
      <summary>Klassen & Lebensformboni</summary>
      <div className="classes">
        <label>
          Spielerklasse
          <select
            aria-label="Spielerklasse"
            value={fleet.playerClass}
            onChange={(event) => {
              const playerClass = event.target.value;
              if (
                playerClass === "none" ||
                playerClass === "collector" ||
                playerClass === "general" ||
                playerClass === "discoverer"
              )
                onChange({ ...fleet, playerClass });
            }}
          >
            <option value="none">Keine</option>
            <option value="collector">Kollektor</option>
            <option value="general">General</option>
            <option value="discoverer">Entdecker</option>
          </select>
        </label>
        <label>
          Allianzklasse
          <select
            aria-label="Allianzklasse"
            value={fleet.allianceClass}
            onChange={(event) => {
              const allianceClass = event.target.value;
              if (
                allianceClass === "none" ||
                allianceClass === "trader" ||
                allianceClass === "warrior" ||
                allianceClass === "researcher"
              )
                onChange({ ...fleet, allianceClass });
            }}
          >
            <option value="none">Keine</option>
            <option value="trader">Händler</option>
            <option value="warrior">Krieger</option>
            <option value="researcher">Forscher</option>
          </select>
        </label>
      </div>
      <p>Lebensformboni in Prozent des Basiswerts (50 = +50 %). Nur Kampfwerte.</p>
      {fleet.units.map((unit) => (
        <fieldset key={unit.id}>
          <legend>{unitName(unit.id)}</legend>
          <div className="tech-fields">
            {STATS.map(([key, label]) => (
              <NumberField
                key={key}
                label={`Lebensform ${unitName(unit.id)} ${label} %`}
                value={unit.lifeform[key]}
                max={1000000}
                integer={false}
                onChange={(value) => {
                  changeLifeform(unit, key, value);
                }}
              />
            ))}
          </div>
        </fieldset>
      ))}
    </details>
  );
}
export function FleetPanel({ attacker, fleet, onChange }: Readonly<Props>) {
  return (
    <section
      className={`fleet-panel ${attacker ? "own" : "enemy"}`}
      aria-label={attacker ? "Deine Flotte" : "Gegner"}
    >
      <header>
        <span className="eyebrow">
          {attacker
            ? "ANGREIFER / AUSWAHL AUS DEINEM SNAPSHOT"
            : "VERTEIDIGER / AUFSTELLUNG"}
        </span>
        <h2>{attacker ? "Deine Flotte" : "Gegner"}</h2>
      </header>
      <UnitPicker attacker={attacker} fleet={fleet} onChange={onChange} />
      {fleet.units.length === 0 ? (
        <p className="empty">Noch keine Einheiten. Ergänze deine Aufstellung manuell.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Einheit</th>
              {attacker && <th>Bestand</th>}
              <th>{attacker ? "Ausgewählt" : "Menge"}</th>
            </tr>
          </thead>
          <tbody>
            {fleet.units.map((unit) => (
              <UnitRow
                key={unit.id}
                unit={unit}
                attacker={attacker}
                change={(changed) => {
                  onChange({
                    ...fleet,
                    units: fleet.units.map((current) =>
                      current.id === unit.id ? changed : current,
                    ),
                  });
                }}
              />
            ))}
          </tbody>
        </table>
      )}
      {attacker && (
        <div className="fleet-actions">
          <button
            type="button"
            onClick={() => {
              onChange(scaleSelection(fleet, 0.5));
            }}
          >
            ½ Menge
          </button>
          <button
            type="button"
            onClick={() => {
              onChange(scaleSelection(fleet, 1.1));
            }}
          >
            +10 %
          </button>
          <button
            type="button"
            onClick={() => {
              onChange(restoreSelection(fleet));
            }}
          >
            Wiederherstellen
          </button>
        </div>
      )}
      <div className="tech-fields">
        {STATS.map(([key, label]) => (
          <NumberField
            key={key}
            label={label}
            value={fleet.technology[key]}
            max={255}
            onChange={(value) => {
              onChange({ ...fleet, technology: { ...fleet.technology, [key]: value } });
            }}
          />
        ))}
      </div>
      <Modifiers fleet={fleet} onChange={onChange} />
    </section>
  );
}
