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

export interface FileNode {
  name: string;
  path: string;
  relative_path: string;
  is_dir: boolean;
  children: FileNode[];
  could_be_nbt: boolean;
}

export interface TypedNbt {
  type:
    | "byte"
    | "short"
    | "int"
    | "long"
    | "float"
    | "double"
    | "string"
    | "list"
    | "compound"
    | "byte_array"
    | "int_array"
    | "long_array";
  value: any;
}
