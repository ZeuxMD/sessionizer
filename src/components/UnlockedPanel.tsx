interface UnlockedPanelProps {
  onHide: () => void;
  onOpenSettings: () => void;
  onReLock: () => void;
}

export function UnlockedPanel({
  onHide,
  onOpenSettings,
  onReLock,
}: UnlockedPanelProps) {
  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-lg w-full text-center">
        <div className="flex items-center justify-center mb-6">
          <svg xmlns="http://www.w3.org/2000/svg" className="h-16 w-16 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.586-3.586A2 2 0 0018.172 5H5.828a2 2 0 00-1.414.586A2 2 0 004 7v10a2 2 0 002 2h12a2 2 0 002-2V7a2 2 0 00-.586-1.414z" />
          </svg>
        </div>

        <h1 className="text-3xl font-bold text-white mb-2">Sessionizer</h1>
        <p className="text-slate-400 mb-8">
          Sessionizer is idle. Use the controls below or close the window to return it to the tray.
        </p>

        <div className="grid gap-3">
          <button
            onClick={onOpenSettings}
            className="w-full bg-blue-600 hover:bg-blue-700 rounded-lg px-6 py-3 font-semibold transition-colors"
          >
            Open Settings
          </button>
          <button
            onClick={onReLock}
            className="w-full bg-slate-700 hover:bg-slate-600 rounded-lg px-6 py-3 font-semibold transition-colors text-white"
          >
            Re-lock Session
          </button>
          <button
            onClick={onHide}
            className="w-full bg-slate-800 hover:bg-slate-700 border border-slate-600 rounded-lg px-6 py-3 font-semibold transition-colors text-white"
          >
            Hide Window
          </button>
        </div>
      </div>
    </div>
  );
}
