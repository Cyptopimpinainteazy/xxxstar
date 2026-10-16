import Link from 'next/link';
import { Home, Search, ArrowLeft } from 'lucide-react';

export default function NotFound() {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center px-4">
      <div className="max-w-md w-full text-center">
        {/* 404 Display */}
        <div className="mb-8">
          <h1 className="text-8xl font-bold bg-gradient-to-r from-orange-500 to-red-500 bg-clip-text text-transparent">
            404
          </h1>
        </div>

        {/* Message */}
        <h2 className="text-2xl font-bold text-white mb-4">
          Page Not Found
        </h2>
        <p className="text-gray-400 mb-8">
          The page you&apos;re looking for doesn&apos;t exist or has been moved.
        </p>

        {/* Action Buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <Link
            href="/"
            className="flex items-center px-6 py-3 rounded-xl bg-gradient-to-r from-orange-600 to-red-600 text-white font-semibold hover:from-orange-500 hover:to-red-500 transition-colors"
          >
            <Home className="w-5 h-5 mr-2" />
            Go Home
          </Link>
          <Link
            href="/explorer"
            className="flex items-center px-6 py-3 rounded-xl bg-[#1a1a1a] border border-[#333333] text-white font-semibold hover:bg-[#222222] transition-colors"
          >
            <Search className="w-5 h-5 mr-2" />
            Explorer
          </Link>
        </div>

        {/* Quick Links */}
        <div className="mt-12 pt-8 border-t border-[#1a1a1a]">
          <p className="text-sm text-gray-500 mb-4">Popular destinations</p>
          <div className="flex flex-wrap justify-center gap-3">
            <Link
              href="/developers/docs"
              className="text-sm text-orange-400 hover:text-orange-300 transition-colors"
            >
              Documentation
            </Link>
            <span className="text-gray-600">•</span>
            <Link
              href="/learn/getting-started"
              className="text-sm text-orange-400 hover:text-orange-300 transition-colors"
            >
              Getting Started
            </Link>
            <span className="text-gray-600">•</span>
            <Link
              href="/network/status"
              className="text-sm text-orange-400 hover:text-orange-300 transition-colors"
            >
              Network Status
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}
