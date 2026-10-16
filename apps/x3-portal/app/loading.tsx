export default function Loading() {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center">
      <div className="text-center">
        {/* Animated Logo/Spinner */}
        <div className="relative w-16 h-16 mx-auto mb-6">
          {/* Outer ring */}
          <div className="absolute inset-0 rounded-full border-4 border-[#1a1a1a]" />
          {/* Spinning gradient ring */}
          <div className="absolute inset-0 rounded-full border-4 border-transparent border-t-orange-500 border-r-red-500 animate-spin" />
          {/* Inner content */}
          <div className="absolute inset-2 rounded-full bg-black flex items-center justify-center">
            <span className="text-lg font-bold bg-gradient-to-r from-orange-400 to-red-400 bg-clip-text text-transparent">
              X3
            </span>
          </div>
        </div>

        {/* Loading text */}
        <p className="text-gray-500 animate-pulse">Loading...</p>
      </div>
    </div>
  );
}
