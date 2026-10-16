'use client';

import React, { useState, useRef } from 'react';
import { motion, useMotionValue, useTransform, useSpring } from 'framer-motion';

// ==================== POSTER ART STYLE PROJECT CARD ====================
interface ProjectCardProps {
  title: string;
  subtitle: string;
  image: string;
  href?: string;
  objectPosition?: string;
}

export function ProjectCard({
  title,
  subtitle,
  image,
  href = '#',
  objectPosition = 'top',
}: ProjectCardProps) {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <motion.a
      href={href}
      className="project-card relative h-[300px] w-full max-w-[600px] flex flex-row cursor-pointer group"
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      whileHover={{ scale: 1.02 }}
      transition={{ duration: 0.2 }}
    >
      {/* Image Container */}
      <div 
        className="relative h-[300px] overflow-hidden rounded-lg transition-all duration-300 ease-in-out"
        style={{ width: isHovered ? '100%' : '75%' }}
      >
        <motion.img
          src={image}
          alt={title}
          className="h-full w-full object-cover transition-transform duration-300"
          style={{ 
            objectPosition,
            transform: isHovered ? 'scale(1.5)' : 'scale(1)',
          }}
        />
        
        {/* Overlay gradient */}
        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-transparent to-slate-900/80 pointer-events-none" />
        
        {/* Scan line effect */}
        <div className="absolute inset-0 pointer-events-none overflow-hidden opacity-30">
          <div 
            className="absolute w-full h-0.5 bg-cyan-400/50"
            style={{ animation: 'scanLine 2s linear infinite' }}
          />
        </div>
      </div>

      {/* Header/Title Container */}
      <div 
        className="absolute right-0 top-0 h-[300px] w-[50%] flex flex-col overflow-hidden transition-all duration-300"
        style={{ transform: isHovered ? 'translateX(20%)' : 'translateX(0)' }}
      >
        {/* Title */}
        <motion.h2 
          className="font-orbitron font-bold text-5xl md:text-7xl text-white uppercase leading-none transition-all duration-300"
          style={{
            transform: isHovered ? 'translateY(-100%) scale(0.4)' : 'translateY(0) scale(1)',
            transformOrigin: 'left top',
          }}
        >
          {title}
        </motion.h2>
        
        {/* Subtitle (revealed on hover) */}
        <motion.h3 
          className="font-orbitron text-2xl md:text-4xl text-cyan-400 uppercase leading-none transition-all duration-300"
          style={{
            transform: isHovered ? 'translateY(-300%) scale(0.4)' : 'translateY(0) scale(1)',
            transformOrigin: 'left top',
          }}
        >
          {subtitle}
        </motion.h3>
      </div>

      {/* Corner accents */}
      <div className="absolute top-2 left-2 w-8 h-8 border-l-2 border-t-2 border-cyan-400/50 transition-all duration-300 group-hover:border-cyan-400" />
      <div className="absolute bottom-2 right-2 w-8 h-8 border-r-2 border-b-2 border-cyan-400/50 transition-all duration-300 group-hover:border-cyan-400" />
    </motion.a>
  );
}

// ==================== PROJECT SHOWCASE GRID ====================
interface Project {
  id: number;
  title: string;
  subtitle: string;
  image: string;
  href?: string;
  objectPosition?: string;
}

const defaultProjects: Project[] = [
  {
    id: 1,
    title: 'Quantum',
    subtitle: 'Core Engine',
    image: 'https://images.unsplash.com/photo-1639762681485-074b7f938ba0?w=800&h=600&fit=crop',
    href: '/quantum',
    objectPosition: 'center',
  },
  {
    id: 2,
    title: 'Neural',
    subtitle: 'AI Swarm',
    image: 'https://images.unsplash.com/photo-1620712943543-bcc4688e7485?w=800&h=600&fit=crop',
    href: '/x3/swarm',
    objectPosition: 'center',
  },
  {
    id: 3,
    title: 'DeFi',
    subtitle: 'Protocols',
    image: 'https://images.unsplash.com/photo-1639762681485-074b7f938ba0?w=800&h=600&fit=crop',
    href: '/defi',
    objectPosition: 'top',
  },
  {
    id: 4,
    title: 'Bridge',
    subtitle: 'Cross-Chain',
    image: 'https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=800&h=600&fit=crop',
    href: '/bridge',
    objectPosition: 'center',
  },
];

interface ProjectShowcaseProps {
  projects?: Project[];
  title?: string;
}

export function ProjectShowcase({
  projects = defaultProjects,
  title = 'EXPLORE ECOSYSTEM',
}: ProjectShowcaseProps) {
  return (
    <div className="w-full py-16">
      {title && (
        <motion.h2 
          className="text-5xl font-orbitron font-bold text-center mb-16"
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
        >
          <span className="bg-gradient-to-r from-cyan-400 via-purple-400 to-pink-400 bg-clip-text text-transparent">
            {title}
          </span>
        </motion.h2>
      )}
      
      <div className="grid md:grid-cols-2 gap-12 max-w-6xl mx-auto px-4">
        {projects.map((project, index) => (
          <motion.div
            key={project.id}
            initial={{ opacity: 0, y: 50 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: index * 0.1 }}
          >
            <ProjectCard {...project} />
          </motion.div>
        ))}
      </div>
    </div>
  );
}

// ==================== RETRO VHS / SCOTCH STYLE FEATURE CARD ====================
interface FeatureCardProps {
  title: string;
  subtitle: string;
  version?: string;
  description: string;
  ctaText?: string;
  ctaHref?: string;
  sphereGradient?: string;
}

export function RetroFeatureCard({
  title,
  subtitle,
  version = 'V1.0',
  description,
  ctaText = 'Learn More',
  ctaHref = '#',
  sphereGradient = 'radial-gradient(circle at left top, #00f3ff, #8b5cf6, #ec4899, #f59e0b, #10b981)',
}: FeatureCardProps) {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <motion.div
      className="retro-card relative overflow-hidden"
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      initial={{ opacity: 0, scale: 0.95 }}
      whileInView={{ opacity: 1, scale: 1 }}
      viewport={{ once: true }}
      style={{
        background: `linear-gradient(
          150deg,
          #0f172a 60.2%,
          #1e293b 60.28%,
          #1e293b 60.7%,
          #0f172a 60.78%,
          #0f172a 61.4%,
          #1e293b 61.48%,
          #1e293b 62.6%,
          #0f172a 62.68%,
          #0f172a 63.4%,
          #1e293b 63.48%,
          #1e293b 64.6%,
          #0f172a 64.68%,
          #0f172a 65.4%,
          #1e293b 65.48%,
          #1e293b 67.4%,
          #0f172a 67.48%,
          #0f172a 68.4%,
          #1e293b 68.48%,
          #1e293b 71.4%,
          #0f172a 71.48%,
          #0f172a 72.4%,
          #1e293b 72.48%,
          #1e293b 76.4%,
          #0f172a 76.48%,
          #0f172a 77.4%,
          #1e293b 77.48%,
          #1e293b 81.4%,
          #0f172a 81.48%,
          #0f172a 82.4%,
          #1e293b 82.48%,
          #1e293b 87.4%,
          #0f172a 87.48%
        )`,
      }}
    >
      {/* Noise texture overlay */}
      <div 
        className="absolute inset-0 pointer-events-none z-10 mix-blend-overlay opacity-30"
        style={{
          backgroundImage: 'url(https://i.gyazo.com/a26366e538851a5c569ff648e99b7fd4.png)',
        }}
      />

      {/* Hover GIF effect */}
      {isHovered && (
        <div 
          className="absolute inset-0 pointer-events-none z-10 mix-blend-screen opacity-40"
          style={{
            backgroundImage: 'url(https://64.media.tumblr.com/da60c13b478dda09ab90c27e880983b8/tumblr_nd4pwdPKdc1tun3l0o1_1280.gifv)',
            backgroundSize: 'cover',
            backgroundPosition: 'center',
          }}
        />
      )}

      {/* Navigation */}
      <nav className="flex items-center justify-end gap-8 p-6 text-sm">
        <a href="/developers/docs" className="text-gray-400 hover:text-white hover:font-bold transition-all">
          DOCS
        </a>
        <a href="/ecosystem" className="text-gray-400 hover:text-white hover:font-bold transition-all">
          ECOSYSTEM
        </a>
        <motion.a 
          href={ctaHref}
          className="px-6 py-2 border border-gray-500 text-gray-400 font-bold hover:bg-white hover:text-slate-900 hover:border-white transition-all"
          whileHover={{ scale: 1.1, rotate: -2 }}
        >
          GET STARTED
        </motion.a>
      </nav>

      {/* Main content */}
      <div className="flex items-center justify-between gap-16 px-16 py-8">
        {/* Text content */}
        <div className="flex flex-col gap-2">
          <h1 className="text-6xl font-bold text-white font-orbitron tracking-tight">
            {title}
          </h1>
          <h2 className="text-lg text-gray-400 font-serif italic">
            {subtitle}
          </h2>
          <h3 className="text-2xl text-gray-500 font-serif">
            {version}
          </h3>
          <p className="max-w-md text-gray-400 mt-4 leading-relaxed">
            {description}
          </p>
          
          <motion.a 
            href={ctaHref}
            className="mt-6 px-8 py-3 w-fit font-bold text-slate-900 transition-all"
            style={{
              background: 'linear-gradient(to right, #ece0c8, #ece0c8)',
            }}
            whileHover={{ 
              scale: 1.1, 
              rotate: 2,
              background: 'linear-gradient(to right, #00f3ff, #8b5cf6, #ec4899)',
              color: '#fff',
            }}
          >
            {ctaText}
          </motion.a>
        </div>

        {/* Sphere */}
        <motion.a 
          href={ctaHref}
          className="block"
          whileHover={{ scale: 1.1 }}
        >
          <motion.div
            className="w-48 h-48 rounded-full border-4 border-slate-900 transition-all duration-300"
            style={{
              background: sphereGradient,
              boxShadow: isHovered ? '16px 16px rgba(0,0,0,0.3)' : '0 0 rgba(0,0,0,0)',
            }}
          />
        </motion.a>
      </div>

      {/* Footer */}
      <div className="text-center py-6">
        <motion.span 
          className="text-white text-sm cursor-pointer"
          whileHover={{ scale: 1.2, fontWeight: 900 }}
        >
          LEARN MORE
        </motion.span>
        <div className="text-white text-3xl -mt-2">↓</div>
      </div>
    </motion.div>
  );
}

// ==================== FEATURE CARDS GRID ====================
interface Feature {
  id: number;
  title: string;
  subtitle: string;
  version?: string;
  description: string;
  ctaText?: string;
  ctaHref?: string;
  sphereGradient?: string;
}

const defaultFeatures: Feature[] = [
  {
    id: 1,
    title: 'Atlas',
    subtitle: 'DUAL-VM BLOCKCHAIN',
    version: 'MAINNET',
    description: 'Designed by the inventors of cross-VM execution, Atlas Sphere gives you superior performance with EVM + SVM side-by-side.',
    ctaText: 'Launch Dapp',
    ctaHref: '/quantum',
    sphereGradient: 'radial-gradient(circle at left top, #00f3ff, #8b5cf6, #ec4899, #f59e0b, #10b981)',
  },
  {
    id: 2,
    title: 'Swarm',
    subtitle: 'GPU COMPUTE NETWORK',
    version: 'V2.0',
    description: 'Distributed AI compute across 10,000+ GPU nodes. Machine learning inference at scale with decentralized verification.',
    ctaText: 'Join Swarm',
    ctaHref: '/x3/swarm',
    sphereGradient: 'radial-gradient(circle at left top, #10b981, #059669, #047857, #065f46, #064e3b)',
  },
];

interface FeatureShowcaseProps {
  features?: Feature[];
  title?: string;
}

export function FeatureShowcase({
  features = defaultFeatures,
  title = 'PLATFORM FEATURES',
}: FeatureShowcaseProps) {
  return (
    <div className="w-full py-16">
      {title && (
        <motion.h2 
          className="text-5xl font-orbitron font-bold text-center mb-16"
          initial={{ opacity: 0, y: 20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
        >
          <span className="bg-gradient-to-r from-green-400 via-cyan-400 to-purple-400 bg-clip-text text-transparent">
            {title}
          </span>
        </motion.h2>
      )}
      
      <div className="flex flex-col gap-16 max-w-5xl mx-auto px-4">
        {features.map((feature, index) => (
          <motion.div
            key={feature.id}
            initial={{ opacity: 0, y: 50 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true }}
            transition={{ delay: index * 0.2 }}
          >
            <RetroFeatureCard {...feature} />
          </motion.div>
        ))}
      </div>
    </div>
  );
}

// ==================== 3D TILT CARD ====================
interface TiltCardProps {
  children: React.ReactNode;
  className?: string;
  glareEnable?: boolean;
  maxTilt?: number;
}

export function TiltCard({
  children,
  className = '',
  glareEnable = true,
  maxTilt = 15,
}: TiltCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  const x = useMotionValue(0);
  const y = useMotionValue(0);

  const rotateX = useTransform(y, [-0.5, 0.5], [maxTilt, -maxTilt]);
  const rotateY = useTransform(x, [-0.5, 0.5], [-maxTilt, maxTilt]);

  const springRotateX = useSpring(rotateX, { stiffness: 300, damping: 30 });
  const springRotateY = useSpring(rotateY, { stiffness: 300, damping: 30 });

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const xPos = (e.clientX - rect.left) / rect.width - 0.5;
    const yPos = (e.clientY - rect.top) / rect.height - 0.5;
    x.set(xPos);
    y.set(yPos);
  };

  const handleMouseLeave = () => {
    x.set(0);
    y.set(0);
  };

  return (
    <motion.div
      ref={ref}
      className={`relative ${className}`}
      style={{
        rotateX: springRotateX,
        rotateY: springRotateY,
        transformStyle: 'preserve-3d',
        perspective: 1000,
      }}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
    >
      {children}
      
      {/* Glare effect */}
      {glareEnable && (
        <motion.div
          className="absolute inset-0 rounded-inherit pointer-events-none"
          style={{
            background: useTransform(
              [x, y],
              ([latestX, latestY]) => {
                const xPercent = ((latestX as number) + 0.5) * 100;
                const yPercent = ((latestY as number) + 0.5) * 100;
                return `radial-gradient(circle at ${xPercent}% ${yPercent}%, rgba(255,255,255,0.15), transparent 50%)`;
              }
            ),
          }}
        />
      )}
    </motion.div>
  );
}

// ==================== FLOATING ACTION CARD ====================
interface FloatingCardProps {
  icon: string;
  title: string;
  description: string;
  href?: string;
  gradient?: string;
}

export function FloatingCard({
  icon,
  title,
  description,
  href = '#',
  gradient = 'from-cyan-500/20 to-purple-500/20',
}: FloatingCardProps) {
  return (
    <motion.a
      href={href}
      className={`block p-8 rounded-2xl border border-cyan-500/30 bg-gradient-to-br ${gradient} backdrop-blur-xl hover:border-cyan-400 transition-all group`}
      whileHover={{ y: -10, scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
    >
      <TiltCard maxTilt={10}>
        <div className="text-5xl mb-4">{icon}</div>
        <h3 className="text-2xl font-orbitron font-bold text-white mb-2 group-hover:text-cyan-400 transition-colors">
          {title}
        </h3>
        <p className="text-gray-400 group-hover:text-gray-300 transition-colors">
          {description}
        </p>
        
        {/* Arrow indicator */}
        <div className="mt-4 text-cyan-400 opacity-0 group-hover:opacity-100 transition-opacity">
          → Explore
        </div>
      </TiltCard>
    </motion.a>
  );
}

// ==================== FLOATING CARDS GRID ====================
interface FloatingCardsGridProps {
  cards?: Array<{
    icon: string;
    title: string;
    description: string;
    href?: string;
    gradient?: string;
  }>;
}

const defaultCards = [
  {
    icon: '⚡',
    title: 'Lightning Fast',
    description: '6-second block time with instant finality powered by GRANDPA consensus.',
    gradient: 'from-yellow-500/20 to-orange-500/20',
  },
  {
    icon: '🔗',
    title: 'Cross-Chain',
    description: 'Seamlessly bridge assets across 103+ blockchain networks.',
    gradient: 'from-blue-500/20 to-cyan-500/20',
  },
  {
    icon: '🧠',
    title: 'AI-Powered',
    description: 'Neural network threat detection and autonomous validation.',
    gradient: 'from-purple-500/20 to-pink-500/20',
  },
  {
    icon: '🛡️',
    title: 'Quantum Safe',
    description: 'Future-proof cryptography resistant to quantum attacks.',
    gradient: 'from-green-500/20 to-emerald-500/20',
  },
];

export function FloatingCardsGrid({ cards = defaultCards }: FloatingCardsGridProps) {
  return (
    <div className="grid md:grid-cols-2 lg:grid-cols-4 gap-6 max-w-7xl mx-auto px-4">
      {cards.map((card, index) => (
        <motion.div
          key={index}
          initial={{ opacity: 0, y: 30 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
          transition={{ delay: index * 0.1 }}
        >
          <FloatingCard {...card} />
        </motion.div>
      ))}
    </div>
  );
}

export default {
  ProjectCard,
  ProjectShowcase,
  RetroFeatureCard,
  FeatureShowcase,
  TiltCard,
  FloatingCard,
  FloatingCardsGrid,
};
