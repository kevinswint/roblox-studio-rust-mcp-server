export type InsertModelArgs = {
	query: string,
}

export type RunCodeArgs = {
	command: string,
}

export type WriteScriptArgs = {
	path: string,
	source: string,
}

export type ReadOutputArgs = {
	filter: string?, -- "all" | "print" | "warn" | "error"
	max_lines: number?, -- Maximum number of lines to return (default: 1000)
	clear_after_read: boolean?, -- Clear buffer after reading (default: true)
}

export type GetStudioStateArgs = {}

export type StartPlaytestArgs = {}

export type StartSimulationArgs = {}

export type StopSimulationArgs = {}

export type StopPlaytestArgs = {}

export type ToolArgs =
	{ InsertModel: InsertModelArgs }
	| { RunCode: RunCodeArgs }
	| { WriteScript: WriteScriptArgs }
	| { ReadOutput: ReadOutputArgs }
	| { GetStudioState: GetStudioStateArgs }
	| { StartPlaytest: StartPlaytestArgs }
	| { StartSimulation: StartSimulationArgs }
	| { StopSimulation: StopSimulationArgs }
	| { StopPlaytest: StopPlaytestArgs }

export type ToolFunction = (ToolArgs) -> string?

return {}
