'use client';

import React, { useRef, useState, useEffect } from 'react';
import { motion, useScroll, useTransform, useSpring, AnimatePresence } from 'framer-motion';
import Image from 'next/image';

// ==================== HORIZONTAL SCROLL TRACK ====================
interface TrackImage {
  id: number;
  src: string;
  alt: string;
  title?: string;
  subtitle?: string;
}

// Default blockchain/tech themed images
const defaultImages: TrackImage[] = [
  { id: 1, src: 'https://images.unsplash.com/photo-1639762681485-074b7f938ba0?w=400&h=400&fit=crop', alt: 'Blockchain', title: 'QUANTUM CHAIN', subtitle: 'Next-gen validation' },
  { id: 2, src: 'https://images.unsplash.com/photo-1518186285589-2f7649de83e0?w=400&h=400&fit=crop', alt: 'Neural Network', title: 'NEURAL NET', subtitle: 'AI-powered security' },
  { id: 3, src: 'https://images.unsplash.com/photo-1558494949-ef010cbdcc31?w=400&h=400&fit=crop', alt: 'Server Room', title: 'DATA CENTERS', subtitle: '147 countries' },
  { id: 4, src: 'https://images.unsplash.com/photo-1550751827-4bd374c3f58b?w=400&h=400&fit=crop', alt: 'Cybersecurity', title: 'QUANTUM SHIELD', subtitle: 'Unbreakable encryption' },
  { id: 5, src: 'https://images.unsplash.com/photo-1516110833967-0b5716ca1387?w=400&h=400&fit=crop', alt: 'GPU', title: 'GPU SWARM', subtitle: 'Distributed compute' },
  { id: 6, src: 'https://images.unsplash.com/photo-1620712943543-bcc4688e7485?w=400&h=400&fit=crop', alt: 'AI', title: 'ATLAS AI', subtitle: 'Autonomous agents' },
  { id: 7, src: 'https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=400&h=400&fit=crop', alt: 'Global Network', title: 'GLOBAL MESH', subtitle: 'Real-time sync' },
  { id: 8, src: 'https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=400&h=400&fit=crop', alt: 'Matrix', title: 'DATA STREAM', subtitle: 'Infinite throughput' },
  { id: 9, src: 'https://images.unsplash.com/photo-1504639725590-34d0984388bd?w=400&h=400&fit=crop', alt: 'Code', title: 'X3 LANG', subtitle: 'Smart contracts 2.0' },
  { id: 10, src: 'https://images.unsplash.com/photo-1563986768494-4dee2763ff3f?w=400&h=400&fit=crop', alt: 'Crypto', title: 'ATLAS TOKEN', subtitle: 'Native currency' },
  { id: 11, src: 'https://images.unsplash.com/photo-1614064641938-3bbee52942c7?w=400&h=400&fit=crop', alt: 'Hologram', title: 'HOLO UI', subtitle: 'AR interface' },
  { id: 12, src: 'https://images.unsplash.com/photo-1677442136019-21780ecad995?w=400&h=400&fit=crop', alt: 'Future City', title: 'METAVERSE', subtitle: 'Digital frontier' },
];

interface HorizontalScrollTrackProps {
  images?: TrackImage[];
  title?: string;
  speed?: number;
  direction?: 'left' | 'right';
  showOverlay?: boolean;
}

export function HorizontalScrollTrack({
  images = defaultImages,
  title = "QUANTUM GALLERY",
  speed = 1,
  direction = 'left',
  showOverlay = true,
}: HorizontalScrollTrackProps) {
  const trackRef = useRef<HTMLDivElement>(null);
  const [hoveredId, setHoveredId] = useState<number | null>(null);
  const [scrollPosition, setScrollPosition] = useState(0);

  // Infinite scroll animation
  useEffect(() => {
    let animationId: number;
    let lastTime = 0;
    
    const animate = (time: number) => {
      if (lastTime) {
        const delta = (time - lastTime) * 0.02 * speed;
        setScrollPosition(prev => {
          const newPos = direction === 'left' ? prev + delta : prev - delta;
          // Reset position when it reaches the end of the first set
          if (Math.abs(newPos) > 100) {
            return 0;
          }
          return newPos;
        });
      }
      lastTime = time;
      animationId = requestAnimationFrame(animate);
    };
    
    if (hoveredId === null) {
      animationId = requestAnimationFrame(animate);
    }
    
    return () => cancelAnimationFrame(animationId);
  }, [hoveredId, speed, direction]);

  // Duplicate images for infinite scroll effect
  const duplicatedImages = [...images, ...images, ...images];

  return (
    <div className="relative py-12 overflow-hidden">
      {/* Title */}
      {title && (
        <motion.h3 
          className="text-4xl font-orbitron font-bold text-center mb-8 bg-gradient-to-r from-cyan-400 via-purple-400 to-pink-400 bg-clip-text text-transparent"
          initial={{ opacity: 0, y: -20 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true }}
        >
          {title}
        </motion.h3>
      )}

      {/* Gradient masks */}
      <div className="absolute inset-0 z-10 pointer-events-none">
        <div className="absolute left-0 top-0 bottom-0 w-32 bg-gradient-to-r from-slate-900 to-transparent" />
        <div className="absolute right-0 top-0 bottom-0 w-32 bg-gradient-to-l from-slate-900 to-transparent" />
      </div>

      {/* Track */}
      <div 
        ref={trackRef}
        className="flex gap-6 px-8"
        style={{
          transform: `translateX(${-scrollPosition}%)`,
          transition: hoveredId !== null ? 'transform 0.3s ease-out' : 'none',
        }}
      >
        {duplicatedImages.map((img, index) => (
          <motion.div
            key={`${img.id}-${index}`}
            className="relative flex-shrink-0 group cursor-pointer"
            onHoverStart={() => setHoveredId(img.id)}
            onHoverEnd={() => setHoveredId(null)}
            whileHover={{ scale: 1.1, zIndex: 20 }}
            transition={{ duration: 0.3 }}
          >
            {/* Card */}
            <div className="relative w-[280px] h-[280px] rounded-2xl overflow-hidden border-2 border-cyan-500/30 shadow-lg shadow-cyan-500/10">
              {/* Image */}
              <img
                src={img.src}
                alt={img.alt}
                className="w-full h-full object-cover transition-transform duration-500 group-hover:scale-110"
              />
              
              {/* Hover Overlay */}
              {showOverlay && (
                <motion.div 
                  className="absolute inset-0 bg-gradient-to-t from-slate-900 via-slate-900/60 to-transparent flex flex-col justify-end p-6"
                  initial={{ opacity: 0 }}
                  whileHover={{ opacity: 1 }}
                >
                  <h4 className="font-orbitron font-bold text-xl text-cyan-400 mb-1">
                    {img.title}
                  </h4>
                  <p className="text-gray-300 text-sm">{img.subtitle}</p>
                </motion.div>
              )}

              {/* Scan Line Effect */}
              <div className="absolute inset-0 pointer-events-none overflow-hidden">
                <div 
                  className="absolute w-full h-1 bg-gradient-to-r from-transparent via-cyan-400/50 to-transparent"
                  style={{
                    animation: 'scanLine 2s linear infinite',
                    top: '-100%',
                  }}
                />
              </div>

              {/* Corner accents */}
              <div className="absolute top-2 left-2 w-6 h-6 border-l-2 border-t-2 border-cyan-400/50" />
              <div className="absolute top-2 right-2 w-6 h-6 border-r-2 border-t-2 border-cyan-400/50" />
              <div className="absolute bottom-2 left-2 w-6 h-6 border-l-2 border-b-2 border-cyan-400/50" />
              <div className="absolute bottom-2 right-2 w-6 h-6 border-r-2 border-b-2 border-cyan-400/50" />
            </div>
          </motion.div>
        ))}
      </div>

      {/* Progress indicator */}
      <div className="flex justify-center mt-8 gap-2">
        {[0, 1, 2, 3, 4].map((i) => (
          <div 
            key={i}
            className="w-2 h-2 rounded-full bg-cyan-500/30"
            style={{
              animation: `pulse 2s ease-in-out ${i * 0.2}s infinite`,
            }}
          />
        ))}
      </div>

      <style jsx>{`
        @keyframes scanLine {
          0% { top: -10%; }
          100% { top: 110%; }
        }
      `}</style>
    </div>
  );
}

// ==================== 3D ROTATING CUBE GALLERY ====================
interface CubeImage {
  id: number;
  src: string;
  alt: string;
}

const cubeImages: CubeImage[] = [
  { id: 1, src: 'https://images.unsplash.com/photo-1639762681485-074b7f938ba0?w=400&h=400&fit=crop', alt: 'Blockchain' },
  { id: 2, src: 'https://images.unsplash.com/photo-1518186285589-2f7649de83e0?w=400&h=400&fit=crop', alt: 'Neural' },
  { id: 3, src: 'https://images.unsplash.com/photo-1558494949-ef010cbdcc31?w=400&h=400&fit=crop', alt: 'Server' },
  { id: 4, src: 'https://images.unsplash.com/photo-1550751827-4bd374c3f58b?w=400&h=400&fit=crop', alt: 'Cyber' },
  { id: 5, src: 'https://images.unsplash.com/photo-1516110833967-0b5716ca1387?w=400&h=400&fit=crop', alt: 'GPU' },
  { id: 6, src: 'https://images.unsplash.com/photo-1620712943543-bcc4688e7485?w=400&h=400&fit=crop', alt: 'AI' },
];

interface RotatingCubeProps {
  images?: CubeImage[];
  size?: number;
  autoRotate?: boolean;
}

export function RotatingCube({
  images = cubeImages,
  size = 300,
  autoRotate = true,
}: RotatingCubeProps) {
  const [rotation, setRotation] = useState({ x: -30, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [bgColor, setBgColor] = useState('#556270');
  const cubeRef = useRef<HTMLDivElement>(null);

  // Auto rotate
  useEffect(() => {
    if (!autoRotate || isDragging) return;
    
    const interval = setInterval(() => {
      setRotation(prev => ({
        x: prev.x,
        y: prev.y + 0.5,
      }));
    }, 50);

    return () => clearInterval(interval);
  }, [autoRotate, isDragging]);

  // Color cycle based on rotation
  useEffect(() => {
    const colors = ['#774F38', '#C5E0DC', '#036564', '#B38184', '#424254', '#4DBCE9'];
    const colorIndex = Math.floor((rotation.y % 360) / 60);
    setBgColor(colors[Math.abs(colorIndex) % colors.length]);
  }, [rotation.y]);

  const handleMouseDown = () => setIsDragging(true);
  const handleMouseUp = () => setIsDragging(false);
  
  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    setRotation(prev => ({
      x: prev.x - e.movementY * 0.5,
      y: prev.y + e.movementX * 0.5,
    }));
  };

  return (
    <div 
      className="relative flex items-center justify-center py-16 rounded-3xl transition-colors duration-500"
      style={{ background: bgColor }}
    >
      <div
        ref={cubeRef}
        className="relative cursor-grab active:cursor-grabbing"
        style={{
          width: size,
          height: size,
          perspective: '1000px',
          transformStyle: 'preserve-3d',
        }}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onMouseMove={handleMouseMove}
      >
        <div
          className="absolute w-full h-full transition-transform duration-100"
          style={{
            transformStyle: 'preserve-3d',
            transform: `rotateX(${rotation.x}deg) rotateY(${rotation.y}deg)`,
          }}
        >
          {/* Front face */}
          <div 
            className="absolute w-full h-full overflow-hidden rounded-lg border-2 border-cyan-400/50"
            style={{ transform: `translateZ(${size/2}px)` }}
          >
            <img src={images[0]?.src} alt={images[0]?.alt} className="w-full h-full object-cover" />
            <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
          </div>
          
          {/* Back face */}
          <div 
            className="absolute w-full h-full overflow-hidden rounded-lg border-2 border-purple-400/50"
            style={{ transform: `rotateY(180deg) translateZ(${size/2}px)` }}
          >
            <img src={images[1]?.src} alt={images[1]?.alt} className="w-full h-full object-cover" />
            <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
          </div>
          
          {/* Right face */}
          <div 
            className="absolute w-full h-full overflow-hidden rounded-lg border-2 border-pink-400/50"
            style={{ transform: `rotateY(90deg) translateZ(${size/2}px)` }}
          >
            <img src={images[2]?.src} alt={images[2]?.alt} className="w-full h-full object-cover" />
            <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
          </div>
          
          {/* Left face */}
          <div 
            className="absolute w-full h-full overflow-hidden rounded-lg border-2 border-green-400/50"
            style={{ transform: `rotateY(-90deg) translateZ(${size/2}px)` }}
          >
            <img src={images[3]?.src} alt={images[3]?.alt} className="w-full h-full object-cover" />
            <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
          </div>
          
          {/* Top face */}
          <div 
            className="absolute w-full h-full overflow-hidden rounded-lg border-2 border-yellow-400/50"
            style={{ transform: `rotateX(90deg) translateZ(${size/2}px)` }}
          >
            <img src={images[4]?.src} alt={images[4]?.alt} className="w-full h-full object-cover" />
            <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
          </div>
          
          {/* Bottom face */}
          <div 
            className="absolute w-full h-full overflow-hidden rounded-lg border-2 border-orange-400/50"
            style={{ transform: `rotateX(-90deg) translateZ(${size/2}px)` }}
          >
            <img src={images[5]?.src} alt={images[5]?.alt} className="w-full h-full object-cover" />
            <div className="absolute inset-0 bg-gradient-to-t from-black/50 to-transparent" />
          </div>
        </div>
      </div>

      {/* Instructions */}
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 text-white/60 text-sm font-mono">
        DRAG TO ROTATE • AUTO-ROTATING
      </div>

      {/* Glow effect */}
      <div 
        className="absolute inset-0 rounded-3xl pointer-events-none"
        style={{
          boxShadow: `inset 0 0 100px ${bgColor}80`,
        }}
      />
    </div>
  );
}

// ==================== MASKED HERO SLIDER ====================
interface Slide {
  id: number;
  title: string[];
  subtitle: string;
  bgImage: string;
  maskImage: string;
  blendColor: string;
}

const defaultSlides: Slide[] = [
  {
    id: 1,
    title: ['QUANTUM', 'CORE'],
    subtitle: 'Next-generation blockchain with dual-VM execution',
    bgImage: 'https://images.unsplash.com/photo-1639762681485-074b7f938ba0?w=1920&h=1080&fit=crop',
    maskImage: 'https://images.unsplash.com/photo-1518186285589-2f7649de83e0?w=1920&h=1080&fit=crop',
    blendColor: '#42605E',
  },
  {
    id: 2,
    title: ['NEURAL', 'NETWORK'],
    subtitle: 'AI-powered validation with self-healing protocols',
    bgImage: 'https://images.unsplash.com/photo-1550751827-4bd374c3f58b?w=1920&h=1080&fit=crop',
    maskImage: 'https://images.unsplash.com/photo-1558494949-ef010cbdcc31?w=1920&h=1080&fit=crop',
    blendColor: '#2D7791',
  },
  {
    id: 3,
    title: ['GPU', 'SWARM'],
    subtitle: 'Distributed compute across 10,000+ nodes',
    bgImage: 'https://images.unsplash.com/photo-1516110833967-0b5716ca1387?w=1920&h=1080&fit=crop',
    maskImage: 'https://images.unsplash.com/photo-1620712943543-bcc4688e7485?w=1920&h=1080&fit=crop',
    blendColor: '#6A0A0D',
  },
  {
    id: 4,
    title: ['ATLAS', 'SPHERE'],
    subtitle: 'The future of decentralized computing',
    bgImage: 'https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=1920&h=1080&fit=crop',
    maskImage: 'https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=1920&h=1080&fit=crop',
    blendColor: '#424254',
  },
];

interface MaskedHeroSliderProps {
  slides?: Slide[];
  autoSlideDelay?: number;
}

export function MaskedHeroSlider({
  slides = defaultSlides,
  autoSlideDelay = 5000,
}: MaskedHeroSliderProps) {
  const [activeSlide, setActiveSlide] = useState(0);
  const [previousSlide, setPreviousSlide] = useState<number | null>(null);
  const [isAnimating, setIsAnimating] = useState(false);

  // Auto slide
  useEffect(() => {
    const timer = setInterval(() => {
      if (!isAnimating) {
        goToSlide((activeSlide + 1) % slides.length);
      }
    }, autoSlideDelay);

    return () => clearInterval(timer);
  }, [activeSlide, isAnimating, slides.length, autoSlideDelay]);

  const goToSlide = (index: number) => {
    if (isAnimating || index === activeSlide) return;
    
    setIsAnimating(true);
    setPreviousSlide(activeSlide);
    setActiveSlide(index);
    
    setTimeout(() => {
      setPreviousSlide(null);
      setIsAnimating(false);
    }, 1500);
  };

  return (
    <div className="relative h-[70vh] min-h-[600px] overflow-hidden rounded-3xl border border-cyan-500/20">
      {/* Slides */}
      {slides.map((slide, index) => (
        <div
          key={slide.id}
          className={`absolute inset-0 transition-all duration-1000 ease-in-out ${
            index === activeSlide 
              ? 'z-10 opacity-100 translate-x-0' 
              : index === previousSlide 
                ? 'z-5 opacity-50 -translate-x-full' 
                : 'z-0 opacity-0 translate-x-full'
          }`}
        >
          {/* Background */}
          <div 
            className="absolute inset-0 bg-cover bg-center transition-transform duration-1000"
            style={{ 
              backgroundImage: `url(${slide.bgImage})`,
              backgroundColor: slide.blendColor,
              backgroundBlendMode: 'luminosity',
              transform: index === activeSlide ? 'translateX(0)' : 'translateX(-20%)',
            }}
          />

          {/* Mask element */}
          <div 
            className="absolute right-[30%] top-[15%] w-[45vh] h-[67vh] overflow-hidden transition-all duration-700"
            style={{
              clipPath: 'polygon(0 0, 100% 0, 100% 100%, 0 100%, 0 0, 6vh 0, 6vh 61vh, 44vh 61vh, 44vh 6vh, 6vh 6vh)',
              opacity: index === activeSlide ? 1 : 0,
              transform: index === activeSlide ? 'rotate(0deg) translateX(0)' : 'rotate(10deg) translateX(200px)',
            }}
          >
            <div 
              className="absolute inset-0 bg-cover bg-center"
              style={{ 
                backgroundImage: `url(${slide.maskImage})`,
                backgroundColor: slide.blendColor,
                backgroundBlendMode: 'luminosity',
                width: '100vw',
                height: '100vh',
                left: '50%',
                top: '50%',
                marginLeft: '-50vw',
                marginTop: '-50vh',
              }}
            />
          </div>

          {/* Content */}
          <div 
            className="absolute left-[40%] top-[40%] z-20"
          >
            <h2 className="mb-4">
              {slide.title.map((line, i) => (
                <div 
                  key={i}
                  className="overflow-hidden"
                  style={{ paddingLeft: i === 1 ? '30px' : 0 }}
                >
                  <span 
                    className={`block text-7xl md:text-9xl font-orbitron font-bold text-white transition-transform duration-1000 delay-300 ${
                      index === activeSlide ? 'translate-y-0' : index === previousSlide ? '-translate-y-full' : 'translate-y-full'
                    }`}
                  >
                    {line}
                  </span>
                </div>
              ))}
            </h2>
            
            <motion.button
              className={`mt-6 ml-[200px] px-6 py-3 text-lg text-white border-l-2 border-r-2 border-white/50 hover:bg-white/10 transition-all font-orbitron uppercase tracking-widest ${
                index === activeSlide ? 'opacity-100' : 'opacity-0'
              }`}
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
            >
              Explore
            </motion.button>
          </div>
        </div>
      ))}

      {/* Navigation */}
      <nav className="absolute bottom-0 right-0 z-30 flex">
        {slides.map((slide, index) => (
          <button
            key={slide.id}
            onClick={() => goToSlide(index)}
            className={`relative w-[120px] h-[50px] text-white text-sm uppercase tracking-wider transition-all ${
              index === activeSlide ? 'bg-slate-900' : 'hover:bg-white/10'
            }`}
            style={{ 
              backgroundColor: index === activeSlide ? '#1F2833' : slide.blendColor,
            }}
          >
            {slide.title[0]}
            
            {/* Progress bar */}
            {index === activeSlide && (
              <span 
                className="absolute bottom-0 left-0 h-0.5 bg-white"
                style={{
                  animation: `progressBar ${autoSlideDelay}ms linear`,
                  width: '100%',
                }}
              />
            )}
          </button>
        ))}
      </nav>

      <style jsx>{`
        @keyframes progressBar {
          from { transform: scaleX(0); transform-origin: left; }
          to { transform: scaleX(1); transform-origin: left; }
        }
      `}</style>
    </div>
  );
}

// ==================== PARALLAX MASKED TEXT ====================
interface SceneContent {
  id: number;
  heading: string;
  title: string;
  description: string;
}

const sceneContents: SceneContent[] = [
  { id: 1, heading: 'Introduction', title: 'A digest of the tutorial', description: 'Atlas Sphere combines cutting-edge blockchain technology with AI-powered validation for unprecedented performance.' },
  { id: 2, heading: 'Navigation', title: 'Hidden radio buttons', description: 'Explore our ecosystem of DeFi protocols, GPU swarms, and cross-chain bridges spanning 103+ networks.' },
  { id: 3, heading: 'Masked Text', title: 'background-clip techniques', description: 'Experience the future with quantum-resistant cryptography and neural network threat detection.' },
  { id: 4, heading: 'Modern CSS', title: 'CSS Grid and Variables', description: 'Built on Substrate with dual-VM execution, Atlas Sphere delivers enterprise-grade blockchain infrastructure.' },
];

interface ParallaxMaskedTextProps {
  scenes?: SceneContent[];
  backgroundText?: string;
  foregroundImage?: string;
  backgroundImage?: string;
}

export function ParallaxMaskedText({
  scenes = sceneContents,
  backgroundText = 'ATLAS',
  foregroundImage = 'https://images.unsplash.com/photo-1451187580459-43490279c0fa?w=4000&h=2660&fit=crop',
  backgroundImage = 'https://images.unsplash.com/photo-1526374965328-7f61d4dc18c5?w=4000&h=2660&fit=crop',
}: ParallaxMaskedTextProps) {
  const [activeScene, setActiveScene] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  
  const { scrollYProgress } = useScroll({
    target: containerRef,
    offset: ["start start", "end end"]
  });

  const textX = useTransform(scrollYProgress, [0, 1], ['40%', '-60%']);
  const bgX = useTransform(scrollYProgress, [0, 1], ['80%', '10%']);

  return (
    <div 
      ref={containerRef}
      className="relative min-h-[400vh] bg-slate-900"
    >
      {/* Sticky container */}
      <div className="sticky top-0 h-screen overflow-hidden">
        {/* Main grid */}
        <div className="grid grid-rows-[3fr_2fr] grid-cols-[5rem_1fr_1fr_5rem] h-full bg-white/5 backdrop-blur-sm">
          
          {/* Landscape section with masked text */}
          <div 
            className="col-span-full row-start-1 row-end-2 flex items-center overflow-hidden bg-cover bg-center"
            style={{ backgroundImage: `url(${backgroundImage})` }}
          >
            <motion.div 
              className="text-[40vh] font-bold whitespace-nowrap"
              style={{ 
                x: textX,
                backgroundImage: `url(${foregroundImage})`,
                backgroundClip: 'text',
                WebkitBackgroundClip: 'text',
                color: 'transparent',
                backgroundSize: '160% auto',
              }}
            >
              {backgroundText}
            </motion.div>
          </div>

          {/* Left side - Page numbers */}
          <div className="row-start-1 row-end-2 col-start-2 col-end-3 self-end text-white font-orbitron">
            <div className="text-6xl relative overflow-hidden">
              <span>0</span>
              <span className="absolute left-[0.5em]">
                {activeScene + 1}
              </span>
              <span className="text-base absolute top-0 left-[4rem]">/0{scenes.length}</span>
            </div>
          </div>

          {/* Hero section */}
          <div className="row-start-1 row-end-2 col-start-3 col-end-5 relative text-white">
            <AnimatePresence mode="wait">
              <motion.div 
                key={activeScene}
                className="absolute inset-0 flex items-center"
                initial={{ opacity: 0, x: 50 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -50 }}
                transition={{ duration: 0.5 }}
              >
                <h1 className="text-6xl font-orbitron font-bold">
                  {scenes[activeScene]?.heading}
                </h1>
              </motion.div>
            </AnimatePresence>

            {/* Navigation */}
            <nav className="absolute bottom-0 right-0 flex gap-1">
              {scenes.map((_, index) => (
                <button
                  key={index}
                  onClick={() => setActiveScene(index)}
                  className={`w-20 h-12 text-white text-sm font-mono transition-all ${
                    index === activeScene ? 'bg-slate-900' : 'bg-white/10 hover:bg-white/20'
                  }`}
                >
                  {index + 1}
                </button>
              ))}
            </nav>
          </div>

          {/* Right content */}
          <div className="row-start-2 row-end-3 col-start-3 col-end-5 p-8 text-white overflow-auto">
            <AnimatePresence mode="wait">
              <motion.div 
                key={activeScene}
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20 }}
                transition={{ duration: 0.5 }}
              >
                <h2 className="text-3xl font-orbitron font-bold mb-4 text-cyan-400">
                  {scenes[activeScene]?.title}
                </h2>
                <p className="text-gray-300 text-lg leading-relaxed">
                  {scenes[activeScene]?.description}
                </p>
              </motion.div>
            </AnimatePresence>
          </div>
        </div>
      </div>
    </div>
  );
}

// Export all components
export default {
  HorizontalScrollTrack,
  RotatingCube,
  MaskedHeroSlider,
  ParallaxMaskedText,
};
