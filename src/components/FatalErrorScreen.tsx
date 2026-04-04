interface FatalErrorScreenProps {
  message: string;
}

export function FatalErrorScreen({ message }: FatalErrorScreenProps) {
  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-lg w-full text-center">
        <div className="flex items-center justify-center mb-6">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-16 w-16 text-red-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v4m0 4h.01M5.07 19h13.86c1.54 0 2.5-1.67 1.73-3L13.73 4c-.77-1.33-2.69-1.33-3.46 0L3.34 16c-.77 1.33.19 3 1.73 3z"
            />
          </svg>
        </div>

        <h1 className="text-3xl font-bold text-white mb-3">Sessionizer</h1>
        <p className="text-red-300 font-semibold mb-4">
          Configuration error detected
        </p>
        <p className="text-slate-300 leading-relaxed">{message}</p>
      </div>
    </div>
  );
}
