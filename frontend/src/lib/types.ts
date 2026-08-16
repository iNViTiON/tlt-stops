export interface ArrivalEntry {
  time: number; // Unix epoch milliseconds, as sent by the API
  isLowEntry?: boolean;
}

export interface StopArrival {
  id: string;
  name: string;
  arrivals: {
    [type: string]: {
      [route: string]: ArrivalEntry[];
    };
  };
}

export interface RawStopArrival {
  id?: string;
  name?: string;
  arrivals?: {
    [type: string]: {
      [route: string]: ArrivalEntry[];
    };
  };
}

export interface FavoriteStop {
  id: string;
  name: string;
}

export type TransportType = string; // e.g., "bus", "tram"

export interface RouteInfo {
  type: TransportType;
  number: string;
}