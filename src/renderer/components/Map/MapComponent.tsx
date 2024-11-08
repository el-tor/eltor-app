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

const fetchGeoLocation = async (ip: string) => {
  const response = await fetch(`https://ipinfo.io/${ip}/geo`);
  const data = await response.json();
  const [latitude, longitude] = data.loc.split(",");
  return [parseFloat(longitude), parseFloat(latitude)];
};

const fetchMyIpAddress = async () => {
  const response = await fetch("https://api.ipify.org?format=json");
  const data = await response.json();
  return data.ip;
};

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const MapComponent = ({
  circuits,
  h,
  scale,
}: {
  circuits: Array<Circuit>;
  h: number;
  scale?: number;
}) => {
  const [markers, setMarkers] = useState<
    { name: string; coordinates: [number, number]; markerOffset: number; ip: string; fingerprint: string }[]
  >([]);

  const fetchMarkers = async () => {
    const myIp = await fetchMyIpAddress();
    const myLocation = await fetchGeoLocation(myIp);
    await delay(1500);
    const ips = circuits[0]?.relayIps ?? [];
    const fingerprints = circuits[0]?.relayFingerprints ?? [];
    const ipLocations = [];
    for (const [index, ip] of ips.entries()) {
      await delay(1500);
      const location = await fetchGeoLocation(ip);
      ipLocations.push({ location, ip, fingerprint: fingerprints[index] });
    }
    const allMarkers = [
      { name: "Me", coordinates: myLocation, markerOffset: -20, ip: myIp, fingerprint: "N/A" },
      ...ipLocations.map(({ location, ip, fingerprint }, index) => ({
        name: `Hop ${index + 1}`,
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
    if (circuits) {
      fetchMarkers();
    }
  }, [circuits]);

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
            key={`${marker.name}-${nextMarker?.name}`}
            from={marker.coordinates}
            to={nextMarker?.coordinates}
            stroke="purple"
            strokeWidth={1.8}
            className="line-animation"
          />
        );
      })}
      {markers.map(({ name, coordinates, markerOffset, ip, fingerprint }, index) => (
        <Tooltip key={name} label={<span>{name}<br />{ip}<br />({fingerprint})</span>} withArrow color="dark">
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
