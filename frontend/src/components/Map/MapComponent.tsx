import {
  ComposableMap,
  Geographies,
  Geography,
  Marker,
  Line,
} from 'react-simple-maps'
import {
  IconHome,
  IconWorld,
  IconNumber1,
  IconNumber2,
  IconNumber3,
} from '@tabler/icons-react'
import { Tooltip } from '@mantine/core'
import features from './countries.json'
import { useEffect, useState, useCallback } from 'react'
import './MapComponent.css'
import { useDispatch, useSelector } from '../../hooks'
import { setMyIp } from '../../globalStore'
// import { useDebounceCallback } from "usehooks-ts";

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

const MapComponent = ({ h, scale }: { h: number; scale?: number }) => {
  const dispatch = useDispatch()
  const { myIp, circuitInUse: circuit } = useSelector((state) => state.global)

  const [markers, setMarkers] = useState<
    {
      hop: number
      name: string
      coordinates: [number, number]
      markerOffset: number
      ip: string
      fingerprint: string
    }[]
  >([])

  const fetchGeoLocation = async (ip: string) => {
    // if (ip.startsWith("127")) {
    return getRandomLatLng()
    // }
    //const geo = await window.electronEvents.lookupIP(ip);
    // return [geo?.ll[1], geo?.ll[0]];
  }

  const fetchMyIpAddress = async () => {
    try {
      const response = await fetch('https://api.ipify.org?format=json')
      const data = await response.json()
      if (data.ip) {
        dispatch(setMyIp(data.ip))
        return data.ip
      } else {
        return myIp
      }
    } catch (e) {
      return myIp
    }
  }

  // Add a small random offset to the coordinates to avoid overlap
  function addRandomOffset([lng, lat]: [number, number]) {
    const offset = () => (Math.random() - 0.95) * 9.5 // up to N deg offset
    return [lng + offset(), lat + offset()] as [number, number]
  }

  const fetchMarkers = useCallback(async () => {
    const myIp = await fetchMyIpAddress()
    const myLocation = await fetchGeoLocation(myIp)
    await delay(1500)
    const ips = circuit.relays.map((i) => i.ip) ?? []
    const fingerprints = circuit.relays.map((f) => f.fingerprint) ?? []
    const relayNames = circuit.relays.map((n) => n.nickname) ?? []
    const ipLocations = []
    for (const [index, ip] of ips.entries()) {
      // await delay(1500);
      const location = await fetchGeoLocation(ip)
      ipLocations.push({
        location,
        ip,
        fingerprint: fingerprints[index],
        name: relayNames[index],
      })
    }
    const allMarkers = [
      {
        name: 'Me',
        coordinates: myLocation,
        //markerOffset: -20,
        ip: myIp,
        fingerprint: 'N/A',
      },
      ...ipLocations.map(({ location, ip, fingerprint, name }, index) => ({
        hop: index + 1,
        name,
        coordinates: addRandomOffset(location as [number, number]),
        //markerOffset: -20,
        ip,
        fingerprint,
      })),
    ]
    // @ts-ignore
    setMarkers(allMarkers)
  }, [circuit])

  // const debouncedFetchMarkers = useDebounceCallback(
  //   fetchMarkers,
  //   5000, // 10 seconds
  //   { leading: true, trailing: false } // Run immediately, drop trailing calls
  // );

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    fetchMarkers()
  }, [])

  return (
    <ComposableMap projectionConfig={{ scale }} height={h}>
      <Geographies geography={features} style={{ pointerEvents: 'none' }}>
        {({ geographies }) =>
          geographies.map((geo) => (
            <Geography key={geo.rsmKey} geography={geo} fill="#333333" />
          ))
        }
      </Geographies>
      {markers.map((marker, i) => {
        if (i === markers.length - 1) return null
        const nextMarker = markers[i + 1]
        return (
          <Line
            key={`${marker.name}-${nextMarker?.name}-${Date.now()}`}
            from={marker.coordinates}
            to={nextMarker?.coordinates}
            stroke="purple"
            strokeWidth={1.8}
            // className="line-animation"
            // style={{ zIndex: 10 }}
          />
        )
      })}
      {markers.map(
        ({ name, coordinates, markerOffset, ip, fingerprint, hop }, index) => (
          <Tooltip
            key={`${Date.now()}${Math.random()}`}
            label={
              <span>
                {name}
                <br />
                {ip}
                <br />({fingerprint})
                <br />({hop ?? 0})
              </span>
            }
            withArrow
            color="dark"
          >
            <Marker coordinates={coordinates}>
              <circle r={9} fill="purple" />
              {index === 0 && (
                <g transform="translate(-6, -6)">
                  <IconHome size={12} color="white" stroke={2} />
                </g>
              )}
              {index === 1 && (
                <g transform="translate(-6, -6)">
                  <IconNumber1 size={12} color="white" stroke={2} />
                </g>
              )}
              {index === 2 && (
                <g transform="translate(-6, -6)">
                  <IconNumber2 size={12} color="white" stroke={2} />
                </g>
              )}
              {index === 3 && (
                <g transform="translate(-6, -6)">
                  <IconNumber3 size={12} color="white" stroke={2} />
                </g>
              )}
              {/* biome-ignore lint/complexity/noUselessFragments: <explanation> */}
              {index === markers.length - 1 && <></>}
            </Marker>
          </Tooltip>
        ),
      )}
    </ComposableMap>
  )
}

export default MapComponent

// List of popular places
const popularPlaces = [
  [-74.006, 40.7128], // New York City, NY, USA
  [-0.1278, 51.5074], // London, UK
  [151.2093, -33.8688], // Sydney, Australia
  [139.6503, 35.6762], // Tokyo, Japan
  [2.3522, 48.8566], // Paris, France
  [-122.4194, 37.7749], // San Francisco, CA, USA
  [-43.1729, -22.9068], // Rio de Janeiro, Brazil
  [-99.1332, 19.4326], // Mexico City, Mexico
  [31.2357, 30.0444], // Cairo, Egypt
  [37.6173, 55.7558], // Moscow, Russia
]

// Function to choose a random coordinate pair
function getRandomLatLng() {
  const randomIndex = Math.floor(Math.random() * popularPlaces.length)
  return popularPlaces[randomIndex]
}
