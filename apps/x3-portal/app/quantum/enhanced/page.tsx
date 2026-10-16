'use client';

import React from 'react';
import dynamic from 'next/dynamic';
import { 
  KineticScroll, 
  StaggeredContainer, 
  StaggeredItem,
  TextReveal,
  AnimatedCounter 
} from '@/components/quantum/KineticScroll';
import { motion, useScroll, useTransform } from 'framer-motion';

// Dynamic imports for heavy components
const QuantumLandingPage = dynamic(() => import('./page'), {
  ssr: false,
  loading: () => (
    <div className="min-h-screen bg-slate-950 flex items-center justify-center">
      <div className="text-center">
        <div className="text-6xl animate-pulse mb-4">⚛️</div>
        <h1 className="text-3xl font-orbitron text-cyan-400">QUANTUM INTERFACE LOADING</h1>
        <div className="flex gap-2 justify-center mt-4">
          {[0, 1, 2, 3, 4].map(i => (
            <div 
              key={i} 
              className="w-3 h-3 bg-cyan-400 rounded-full animate-bounce" 
              style={{ animationDelay: `${i * 0.1}s` }} 
            />
          ))}
        </div>
      </div>
    </div>
  )
});

const NeuralThreatDetector = dynamic(
  () => import('@/components/quantum/NeuralThreatDetector'),
  { ssr: false }
);

export default function QuantumEnhancedPage() {
  const { scrollYProgress } = useScroll();
  const backgroundY = useTransform(scrollYProgress, [0, 1], ['0%', '50%']);
  
  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-purple-950/20 to-slate-950 overflow-hidden relative">
      {/* Animated Background Gradient */}
      <motion.div 
        className="fixed inset-0 pointer-events-none z-0"
        style={{ y: backgroundY }}
      >
        <div className="absolute top-0 left-1/4 w-96 h-96 bg-cyan-500/10 rounded-full blur-3xl" />
        <div className="absolute top-1/2 right-1/4 w-96 h-96 bg-purple-500/10 rounded-full blur-3xl" />
        <div className="absolute bottom-0 left-1/2 w-96 h-96 bg-pink-500/10 rounded-full blur-3xl" />
      </motion.div>
      
      {/* Hero Section with Kinetic Effects */}
      <section className="relative min-h-screen flex flex-col items-center justify-center px-4 overflow-hidden">
        <KineticScroll effect="zoom" intensity={1.2}>
          <motion.h1 
            className="text-7xl md:text-9xl font-orbitron font-black text-center mb-8"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
          >
            <span className="bg-gradient-to-r from-cyan-400 via-purple-400 to-pink-400 bg-clip-text text-transparent glitch-text">
              QUANTUM
            </span>
          </motion.h1>
        </KineticScroll>
        
        <KineticScroll effect="slide" direction="up" delay={0.3}>
          <h2 className="text-4xl md:text-6xl font-orbitron font-bold text-center text-white mb-6">
            VALIDATOR NETWORK
          </h2>
        </KineticScroll>
        
        <KineticScroll effect="fade" delay={0.5}>
          <p className="text-xl md:text-2xl text-gray-300 max-w-4xl text-center mx-auto">
            <TextReveal 
              text="Experience the next evolution in blockchain monitoring. Real-time AI-powered validation across 147 countries." 
              delay={0.8}
            />
          </p>
        </KineticScroll>
        
        {/* Animated Stats */}
        <StaggeredContainer className="grid grid-cols-2 md:grid-cols-4 gap-6 mt-16 max-w-5xl mx-auto" staggerDelay={0.15}>
          <StaggeredItem effect="scale">
            <div className="bg-slate-900/80 backdrop-blur-xl border border-cyan-500/30 rounded-2xl p-6 text-center">
              <div className="text-4xl mb-2">🌐</div>
              <div className="font-orbitron text-3xl text-cyan-400">
                <AnimatedCounter to={147} suffix="+" />
              </div>
              <div className="text-sm text-gray-400 uppercase tracking-widest">Validators</div>
            </div>
          </StaggeredItem>
          
          <StaggeredItem effect="scale">
            <div className="bg-slate-900/80 backdrop-blur-xl border border-green-500/30 rounded-2xl p-6 text-center">
              <div className="text-4xl mb-2">⚡</div>
              <div className="font-orbitron text-3xl text-green-400">
                <AnimatedCounter to={99} suffix="%" prefix="" />
              </div>
              <div className="text-sm text-gray-400 uppercase tracking-widest">Uptime</div>
            </div>
          </StaggeredItem>
          
          <StaggeredItem effect="scale">
            <div className="bg-slate-900/80 backdrop-blur-xl border border-purple-500/30 rounded-2xl p-6 text-center">
              <div className="text-4xl mb-2">📦</div>
              <div className="font-orbitron text-3xl text-purple-400">
                <AnimatedCounter to={8924517} />
              </div>
              <div className="text-sm text-gray-400 uppercase tracking-widest">Blocks</div>
            </div>
          </StaggeredItem>
          
          <StaggeredItem effect="scale">
            <div className="bg-slate-900/80 backdrop-blur-xl border border-pink-500/30 rounded-2xl p-6 text-center">
              <div className="text-4xl mb-2">🛡️</div>
              <div className="font-orbitron text-3xl text-pink-400">0</div>
              <div className="text-sm text-gray-400 uppercase tracking-widest">Breaches</div>
            </div>
          </StaggeredItem>
        </StaggeredContainer>
        
        {/* Scroll Indicator */}
        <motion.div 
          className="absolute bottom-10"
          animate={{ y: [0, 10, 0] }}
          transition={{ duration: 2, repeat: Infinity }}
        >
          <div className="w-6 h-10 border-2 border-cyan-500/50 rounded-full flex justify-center">
            <div className="w-1.5 h-3 bg-cyan-400 rounded-full mt-2 animate-bounce" />
          </div>
        </motion.div>
      </section>
      
      {/* Feature Section with Explode Effect */}
      <section className="relative py-32 px-4">
        <KineticScroll effect="spin" intensity={0.5}>
          <h2 className="text-5xl md:text-7xl font-orbitron font-bold text-center mb-4">
            <span className="bg-gradient-to-r from-green-400 via-cyan-400 to-blue-400 bg-clip-text text-transparent">
              FEATURES
            </span>
          </h2>
        </KineticScroll>
        
        <KineticScroll effect="fade">
          <p className="text-xl text-gray-400 text-center max-w-2xl mx-auto mb-16">
            Next-generation blockchain infrastructure powered by quantum-resistant algorithms
          </p>
        </KineticScroll>
        
        <div className="max-w-7xl mx-auto grid md:grid-cols-3 gap-8">
          <KineticScroll effect="slide" direction="left">
            <FeatureCard 
              icon="🧠"
              title="Neural Consensus"
              description="AI-driven validation that adapts to network conditions in real-time"
              color="cyan"
            />
          </KineticScroll>
          
          <KineticScroll effect="zoom">
            <FeatureCard 
              icon="⚛️"
              title="Quantum Secure"
              description="Post-quantum cryptography protects against future threats"
              color="purple"
            />
          </KineticScroll>
          
          <KineticScroll effect="slide" direction="right">
            <FeatureCard 
              icon="⚡"
              title="Lightning Fast"
              description="Sub-second finality with 1000x energy efficiency"
              color="yellow"
            />
          </KineticScroll>
          
          <KineticScroll effect="explode">
            <FeatureCard 
              icon="🌐"
              title="Global Mesh"
              description="Distributed across 147 countries with automatic failover"
              color="green"
            />
          </KineticScroll>
          
          <KineticScroll effect="flip">
            <FeatureCard 
              icon="🔐"
              title="Zero Knowledge"
              description="Private transactions with full auditability"
              color="pink"
            />
          </KineticScroll>
          
          <KineticScroll effect="wave">
            <FeatureCard 
              icon="∞"
              title="Infinite Scale"
              description="Horizontal scaling to millions of transactions"
              color="orange"
            />
          </KineticScroll>
        </div>
      </section>
      
      {/* Neural Threat Detector Section */}
      <section className="relative py-32 px-4">
        <KineticScroll effect="reveal">
          <h2 className="text-5xl font-orbitron font-bold text-center mb-16">
            <span className="bg-gradient-to-r from-red-400 via-orange-400 to-yellow-400 bg-clip-text text-transparent">
              THREAT INTELLIGENCE
            </span>
          </h2>
        </KineticScroll>
        
        <KineticScroll effect="zoom" intensity={0.8}>
          <div className="max-w-5xl mx-auto">
            <NeuralThreatDetector />
          </div>
        </KineticScroll>
      </section>
      
      {/* Call to Action */}
      <section className="relative py-32 px-4">
        <KineticScroll effect="explode" intensity={0.6}>
          <div className="max-w-5xl mx-auto bg-gradient-to-br from-cyan-500/10 via-purple-500/10 to-pink-500/10 rounded-3xl border border-cyan-500/30 p-12 text-center backdrop-blur-xl">
            <h2 className="text-5xl font-orbitron font-bold text-white mb-6">
              Ready to Experience the Future?
            </h2>
            <p className="text-xl text-gray-300 mb-8 max-w-3xl mx-auto">
              Join the most advanced validator network with AI-powered optimization 
              and quantum-resistant security.
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <motion.button 
                className="px-12 py-5 bg-gradient-to-r from-cyan-500 to-purple-500 rounded-xl font-orbitron font-bold text-xl hover:shadow-lg hover:shadow-cyan-500/30 transition-all"
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
              >
                🚀 GET STARTED
              </motion.button>
              <motion.button 
                className="px-12 py-5 border border-purple-500/50 rounded-xl font-orbitron text-purple-400 hover:bg-purple-500/10 transition-all"
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
              >
                📖 READ DOCS
              </motion.button>
            </div>
          </div>
        </KineticScroll>
      </section>
      
      {/* Footer */}
      <footer className="border-t border-slate-800 py-12 px-4">
        <div className="max-w-7xl mx-auto flex flex-col md:flex-row justify-between items-center">
          <div className="text-3xl font-orbitron font-bold bg-gradient-to-r from-cyan-400 to-purple-400 bg-clip-text text-transparent mb-4 md:mb-0">
            QUANTUM::NETWORK
          </div>
          <div className="flex gap-8 text-gray-400">
            <a href="#" className="hover:text-cyan-400 transition-colors">Docs</a>
            <a href="#" className="hover:text-cyan-400 transition-colors">Ecosystem</a>
            <a href="#" className="hover:text-cyan-400 transition-colors">AI Swarm</a>
            <a href="#" className="hover:text-cyan-400 transition-colors">GitHub</a>
          </div>
        </div>
        <div className="text-center text-gray-600 mt-8 text-sm">
          © 2060 Quantum Validator Network. All rights reserved.
        </div>
      </footer>
    </div>
  );
}

// Feature Card Component
function FeatureCard({ 
  icon, 
  title, 
  description, 
  color 
}: { 
  icon: string; 
  title: string; 
  description: string; 
  color: string;
}) {
  const colorClasses: Record<string, string> = {
    cyan: 'border-cyan-500/30 hover:border-cyan-500/50 from-cyan-500/10',
    purple: 'border-purple-500/30 hover:border-purple-500/50 from-purple-500/10',
    yellow: 'border-yellow-500/30 hover:border-yellow-500/50 from-yellow-500/10',
    green: 'border-green-500/30 hover:border-green-500/50 from-green-500/10',
    pink: 'border-pink-500/30 hover:border-pink-500/50 from-pink-500/10',
    orange: 'border-orange-500/30 hover:border-orange-500/50 from-orange-500/10',
  };
  
  return (
    <motion.div
      className={`bg-gradient-to-br ${colorClasses[color]} to-transparent rounded-3xl border p-8 backdrop-blur-xl transition-all duration-300`}
      whileHover={{ scale: 1.05, y: -10 }}
    >
      <div className="text-6xl mb-6">{icon}</div>
      <h3 className="text-2xl font-orbitron font-bold text-white mb-4">{title}</h3>
      <p className="text-gray-400">{description}</p>
    </motion.div>
  );
}
