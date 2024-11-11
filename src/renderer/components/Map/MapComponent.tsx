import {
  ComposableMap,
  Geographies,
  Geography,
  Marker,
  Line,
} from "react-simple-maps";
import { IconHome, IconWorld } from "@tabler/icons-react";
import { Tooltip } from '@mantine/core';

import features from "./countries.json";
import { type Circuit } from "renderer/globalStore";
import { useEffect, useState } from "react";
import "./MapComponent.css";
import { type CircuitRenew } from "main/tor/circuitRenewWatcher";

const fetchGeoLocation = async (ip: string) => {
  // TODO: cache this
  const response = await fetch(`https://ipinfo.io/${ip}/geo`);
  const data = await response.json();
  const [latitude, longitude] = data.loc.split(",");
  return [parseFloat(longitude), parseFloat(latitude)];
};

const fetchMyIpAddress = async () => {
  // TODO: cache this
  const response = await fetch("https://api.ipify.org?format=json");
  const data = await response.json();
  return data.ip;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const MapComponent = ({
  circuit,
  h,
  scale,
}: {
  circuit: CircuitRenew;
  h: number;
  scale?: number;
}) => {
  const [markers, setMarkers] = useState<
    { hop: number, name: string; coordinates: [number, number]; markerOffset: number; ip: string; fingerprint: string }[]
  >([]);

  const fetchMarkers = async () => {
    const myIp = await fetchMyIpAddress();
    const myLocation = await fetchGeoLocation(myIp);
    await delay(1500);
    const ips = circuit.relays.map(i=>i.ip) ?? [];
    const fingerprints = circuit.relays.map(f=>f.fingerprint) ?? [];
    const relayNames = circuit.relays.map(n=>n.nickname) ?? [];
    const ipLocations = [];
    for (const [index, ip] of ips.entries()) {
      await delay(1500);
      const location = await fetchGeoLocation(ip);
      ipLocations.push({ location, ip, fingerprint: fingerprints[index], name: relayNames[index] });
    }
    const allMarkers = [
      { name: "Me", coordinates: myLocation, markerOffset: -20, ip: myIp, fingerprint: "N/A" },
      ...ipLocations.map(({ location, ip, fingerprint, name }, index) => ({
        hop: index + 1,
        name,
        coordinates: location,
        markerOffset: -20,
        ip,
        fingerprint,
      })),
    ];
    // @ts-ignore
    setMarkers(allMarkers);
  };

  useEffect(() => {
    if (circuit) {
      fetchMarkers();
    }
  }, [circuit]);

  return (
    <ComposableMap projectionConfig={{ scale }} height={h}>
      <Geographies geography={features} style={{ pointerEvents: "none" }}>
        {({ geographies }) =>
          geographies.map((geo) => (
            <Geography key={geo.rsmKey} geography={geo} fill="#333333" />
          ))
        }
      </Geographies>
      {markers.map((marker, i) => {
        if (i === markers.length - 1) return null;
        const nextMarker = markers[i + 1];
        return (
          <Line
            key={`${marker.name}-${nextMarker?.name}-${Date.now()}`}
            from={marker.coordinates}
            to={nextMarker?.coordinates}
            stroke="purple"
            strokeWidth={1.8}
            className="line-animation"
          />
        );
      })}
      {markers.map(({ name, coordinates, markerOffset, ip, fingerprint }, index) => (
        <Tooltip key={`${Date.now()}${Math.random()}`} label={<span>{name}<br />{ip}<br />({fingerprint})</span>} withArrow color="dark">
          <Marker coordinates={coordinates} className="marker-animation">
            <circle r={9} fill="purple" />
            {index === 0 && (
              <g transform="translate(-6, -6)">
                <IconHome size={12} color="white" stroke={2} />
              </g>
            )}
            {index === markers.length - 1 && (
              <g transform="translate(-6, -6)">
                <IconWorld size={12} color="white" stroke={2} />
              </g>
            )}
          </Marker>
        </Tooltip>
      ))}
    </ComposableMap>
  );
};

export default MapComponent;
