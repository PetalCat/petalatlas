export interface WorldInfo {
  id: string;
  name: string;
  folder_name: string;
  path: string;
  version_id: string;
  last_played: number;
  size: number;
  icon: string | null;
  seed: number | null;
  game_mode: string;
  platform: string;
  is_zip: boolean;
}
