'use client';

import React from 'react';
import Link from 'next/link';
import {
  GraduationCap,
  PlayCircle,
  FileText,
  Award,
  BookOpen,
  Code,
  Rocket,
  Users,
  CheckCircle2,
  ArrowRight,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const learningPaths = [
  {
    title: 'Blockchain Fundamentals',
    description: 'Learn the basics of blockchain technology and X3 STAR architecture',
    duration: '4 hours',
    modules: 8,
    level: 'Beginner',
    topics: ['What is blockchain?', 'Consensus mechanisms', 'Smart contracts basics', 'X3 STAR architecture'],
  },
  {
    title: 'EVM Development',
    description: 'Build dApps using Solidity and EVM tools',
    duration: '8 hours',
    modules: 12,
    level: 'Intermediate',
    topics: ['Solidity fundamentals', 'Hardhat setup', 'Testing contracts', 'Deploying to X3'],
  },
  {
    title: 'SVM Development',
    description: 'Build programs using Rust and the Solana Virtual Machine',
    duration: '10 hours',
    modules: 14,
    level: 'Intermediate',
    topics: ['Rust basics', 'Anchor framework', 'Program architecture', 'Cross-program invocation'],
  },
  {
    title: 'Cross-VM Development',
    description: 'Master atomic cross-VM transactions with Comits',
    duration: '6 hours',
    modules: 8,
    level: 'Advanced',
    topics: ['Comit architecture', 'Dual-payload design', 'State synchronization', 'Error handling'],
  },
];

const certifications = [
  { name: 'X3 STAR Certified Developer', level: 'Associate', examDuration: '2 hours' },
  { name: 'Cross-VM Specialist', level: 'Professional', examDuration: '3 hours' },
  { name: 'X3 STAR Architect', level: 'Expert', examDuration: '4 hours' },
];

const resources = [
  {
    title: 'Video Tutorials',
    description: 'Step-by-step video guides',
    icon: <PlayCircle className="w-6 h-6" />,
    count: '50+ videos',
  },
  {
    title: 'Written Guides',
    description: 'In-depth documentation',
    icon: <FileText className="w-6 h-6" />,
    count: '100+ articles',
  },
  {
    title: 'Code Examples',
    description: 'Production-ready code',
    icon: <Code className="w-6 h-6" />,
    count: '200+ snippets',
  },
  {
    title: 'Community',
    description: 'Learn with others',
    icon: <Users className="w-6 h-6" />,
    count: '5K+ members',
  },
];

export default function LearningPage() {
  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <Link href="/developers" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Developers
            </Link>
            <div className="badge badge-info mt-4 mb-4">Learning Center</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Developer Learning Center
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Master X3 STAR development with structured learning paths, 
              hands-on tutorials, and professional certifications.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#paths" className="btn-primary">
                Start Learning
              </a>
              <a href="#certifications" className="btn-secondary">
                Get Certified
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Resources */}
      <section className="py-12 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {resources.map((resource, index) => (
              <div key={index} className="glass-card p-6 text-center">
                <div className="p-3 rounded-xl bg-orange-500/20 text-orange-400 w-fit mx-auto mb-4">
                  {resource.icon}
                </div>
                <h3 className="font-semibold text-white mb-1">{resource.title}</h3>
                <p className="text-sm text-gray-400 mb-2">{resource.description}</p>
                <span className="text-cyan-400 font-medium">{resource.count}</span>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Learning Paths */}
      <section id="paths" className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Learning Paths</h2>
          
          <div className="space-y-6">
            {learningPaths.map((path, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="text-xl font-semibold text-white">{path.title}</h3>
                      <span className={`badge ${path.level === 'Beginner' ? 'badge-success' : path.level === 'Intermediate' ? 'badge-warning' : 'badge-error'}`}>
                        {path.level}
                      </span>
                    </div>
                    <p className="text-gray-400 mb-3">{path.description}</p>
                    <div className="flex flex-wrap gap-2">
                      {path.topics.map((topic, i) => (
                        <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                          {topic}
                        </span>
                      ))}
                    </div>
                  </div>
                  <div className="flex items-center gap-6">
                    <div className="text-center">
                      <p className="text-2xl font-bold text-white">{path.modules}</p>
                      <p className="text-sm text-gray-400">Modules</p>
                    </div>
                    <div className="text-center">
                      <p className="text-2xl font-bold text-white">{path.duration}</p>
                      <p className="text-sm text-gray-400">Duration</p>
                    </div>
                    <button className="btn-primary whitespace-nowrap">
                      Start Path
                      <ArrowRight className="w-4 h-4 ml-2" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Certifications */}
      <section id="certifications" className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-2xl font-bold text-white">Certifications</h2>
            <Award className="w-8 h-8 text-amber-400" />
          </div>
          
          <div className="grid md:grid-cols-3 gap-6">
            {certifications.map((cert, index) => (
              <div key={index} className="glass-card p-6 text-center">
                <div className="w-16 h-16 rounded-full bg-gradient-to-br from-orange-500 to-amber-500 flex items-center justify-center mx-auto mb-4">
                  <Award className="w-8 h-8 text-white" />
                </div>
                <h3 className="text-lg font-semibold text-white mb-2">{cert.name}</h3>
                <p className="text-gray-400 mb-4">
                  Level: <span className="text-white">{cert.level}</span>
                </p>
                <p className="text-sm text-gray-400 mb-4">Exam: {cert.examDuration}</p>
                <button className="btn-secondary w-full">Learn More</button>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Progress Tracking */}
      <section className="py-16">
        <div className="container-wide">
          <div className="glass-card p-8">
            <div className="flex flex-col md:flex-row items-center justify-between gap-6">
              <div>
                <h2 className="text-2xl font-bold text-white mb-2">Track Your Progress</h2>
                <p className="text-gray-400">
                  Sign in to track your learning progress, earn badges, and get personalized recommendations.
                </p>
              </div>
              <button className="btn-primary whitespace-nowrap">
                Sign In to Track
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <GraduationCap className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Start Your Learning Journey
          </h2>
          <p className="text-gray-400 mb-8">
            From beginner to expert, we have resources for every skill level.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <a href="#paths" className="btn-primary">
              Browse Courses
            </a>
            <Link href="/community" className="btn-secondary">
              Join Community
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
