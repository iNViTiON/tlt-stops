export interface StopArrival {
  id: string;
  name: string;
  arrivals: {
    [type: string]: {
      [route: string]: Array<{
        time: number; // Timestamp in milliseconds
        timeString?: string; // Original ISO string if needed for display
        isLowEntry?: boolean;
      }>;
    };
  };
}

export interface RawStopArrival {
  id?: string;
  name?: string;
  arrivals?: {
    [type: string]: {
      [route: string]: Array<{
        time: string; // ISO string from API
        isLowEntry?: boolean;
      }>;
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