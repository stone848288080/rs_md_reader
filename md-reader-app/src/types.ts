export interface OpenedFile {
  /** Absolute path of the opened file. */
  path: string;
  /** File name only, shown in the window title. */
  name: string;
  content: string;
  /** Directory used to resolve relative image paths. */
  baseDir: string;
  /** File size in bytes. */
  size: number;
}
