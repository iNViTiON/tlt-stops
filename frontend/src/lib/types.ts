export interface StopArrival {
  id: string;
  name: string;
  arrivals: {
    [type: string]: {
      [route: string]: Array<{
        time: string;
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