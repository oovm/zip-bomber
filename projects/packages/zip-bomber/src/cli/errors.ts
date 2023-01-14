/** Host-side CLI input / I/O failures with catalog message ids. */
export class InputError extends Error {
    constructor(
        public readonly messageId: string,
        public readonly args: Record<string, string> = {},
    ) {
        super(messageId);
    }
}
