'use client';

import React from 'react';
import Link from 'next/link';
import {
  Calendar,
  MapPin,
  Users,
  Video,
  Mic,
  Clock,
  ExternalLink,
  ArrowRight,
  Globe,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const upcomingEvents = [
  {
    title: 'X3 STAR Developer Summit 2024',
    type: 'Conference',
    date: 'March 15-17, 2024',
    location: 'San Francisco, CA',
    mode: 'In-Person',
    description: 'Annual developer conference featuring workshops, talks, and networking',
    attendees: 500,
    featured: true,
  },
  {
    title: 'Building Cross-VM dApps',
    type: 'Workshop',
    date: 'February 28, 2024',
    location: 'Online',
    mode: 'Virtual',
    description: 'Hands-on workshop on building atomic cross-VM transactions',
    attendees: 200,
    featured: false,
  },
  {
    title: 'X3 Hackathon: DeFi Track',
    type: 'Hackathon',
    date: 'March 1-3, 2024',
    location: 'Online',
    mode: 'Virtual',
    description: '48-hour hackathon focused on DeFi innovation',
    attendees: 350,
    prize: '$50,000',
    featured: false,
  },
  {
    title: 'Community Town Hall',
    type: 'Community',
    date: 'Last Friday of Month',
    location: 'Discord',
    mode: 'Virtual',
    description: 'Monthly community call with team updates and Q&A',
    attendees: 150,
    recurring: true,
    featured: false,
  },
];

const pastEvents = [
  {
    title: 'Mainnet Launch Party',
    date: 'January 2024',
    location: 'Multiple Cities',
    recording: true,
  },
  {
    title: 'Token Extensions Deep Dive',
    date: 'December 2023',
    location: 'Online',
    recording: true,
  },
  {
    title: 'ETH Denver Side Event',
    date: 'February 2023',
    location: 'Denver, CO',
    recording: false,
  },
];

const eventTypes = [
  { name: 'Conferences', count: 4, icon: <Mic className="w-5 h-5" /> },
  { name: 'Workshops', count: 12, icon: <Users className="w-5 h-5" /> },
  { name: 'Hackathons', count: 6, icon: <Calendar className="w-5 h-5" /> },
  { name: 'Meetups', count: 25, icon: <MapPin className="w-5 h-5" /> },
];

export default function EventsPage() {
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
            <Link href="/community" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Community
            </Link>
            <div className="badge badge-info mt-4 mb-4">Events</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Events & Meetups
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Join the X3 STAR community at conferences, workshops, hackathons, 
              and local meetups around the world.
            </p>
            <div className="flex flex-wrap gap-4">
              <a href="#upcoming" className="btn-primary">
                View Upcoming
              </a>
              <a href="#host" className="btn-secondary">
                Host an Event
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* Event Types */}
      <section className="py-8 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-4 gap-6">
            {eventTypes.map((type, index) => (
              <div key={index} className="flex items-center gap-4">
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400">
                  {type.icon}
                </div>
                <div>
                  <p className="text-2xl font-bold text-white">{type.count}</p>
                  <p className="text-sm text-gray-400">{type.name}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Featured Event */}
      {upcomingEvents.filter(e => e.featured).map((event, index) => (
        <section key={index} className="py-16">
          <div className="container-wide">
            <div className="glass-card p-8 border-orange-500/30">
              <div className="flex items-center gap-2 mb-4">
                <span className="badge badge-warning">Featured</span>
                <span className="badge badge-default">{event.type}</span>
              </div>
              <h2 className="text-3xl font-bold text-white mb-4">{event.title}</h2>
              <p className="text-gray-400 mb-6">{event.description}</p>
              <div className="grid sm:grid-cols-3 gap-6 mb-6">
                <div className="flex items-center text-gray-400">
                  <Calendar className="w-5 h-5 text-orange-400 mr-3" />
                  {event.date}
                </div>
                <div className="flex items-center text-gray-400">
                  <MapPin className="w-5 h-5 text-orange-400 mr-3" />
                  {event.location}
                </div>
                <div className="flex items-center text-gray-400">
                  <Users className="w-5 h-5 text-orange-400 mr-3" />
                  {event.attendees}+ Expected
                </div>
              </div>
              <button className="btn-primary">
                Register Now
                <ExternalLink className="w-4 h-4 ml-2" />
              </button>
            </div>
          </div>
        </section>
      ))}

      {/* Upcoming Events */}
      <section id="upcoming" className="py-16 bg-[#050505]">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Upcoming Events</h2>
          
          <div className="space-y-4">
            {upcomingEvents.filter(e => !e.featured).map((event, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="badge badge-default">{event.type}</span>
                      {event.mode === 'Virtual' && (
                        <span className="badge badge-info">Virtual</span>
                      )}
                      {event.recurring && (
                        <span className="badge badge-success">Recurring</span>
                      )}
                    </div>
                    <h3 className="text-xl font-semibold text-white mb-2">{event.title}</h3>
                    <p className="text-gray-400 text-sm mb-3">{event.description}</p>
                    <div className="flex flex-wrap items-center gap-4 text-sm text-gray-400">
                      <span className="flex items-center">
                        <Calendar className="w-4 h-4 mr-1" />
                        {event.date}
                      </span>
                      <span className="flex items-center">
                        <MapPin className="w-4 h-4 mr-1" />
                        {event.location}
                      </span>
                      {event.prize && (
                        <span className="text-emerald-400 font-medium">
                          Prize Pool: {event.prize}
                        </span>
                      )}
                    </div>
                  </div>
                  <button className="btn-secondary whitespace-nowrap">
                    Learn More
                    <ArrowRight className="w-4 h-4 ml-2" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Past Events */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">Past Events</h2>
          
          <div className="grid md:grid-cols-3 gap-6">
            {pastEvents.map((event, index) => (
              <div key={index} className="glass-card p-6">
                <h3 className="font-semibold text-white mb-2">{event.title}</h3>
                <div className="flex items-center gap-4 text-sm text-gray-400 mb-4">
                  <span>{event.date}</span>
                  <span>{event.location}</span>
                </div>
                {event.recording ? (
                  <button className="text-orange-400 hover:text-orange-300 text-sm flex items-center">
                    <Video className="w-4 h-4 mr-1" />
                    Watch Recording
                  </button>
                ) : (
                  <span className="text-sm text-gray-500">No recording available</span>
                )}
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Host Event */}
      <section id="host" className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Host a Meetup</h2>
              <p className="text-gray-400 mb-6">
                Want to organize an X3 STAR event in your city? We provide resources, 
                swag, and support for community-led meetups.
              </p>
              <ul className="space-y-3 mb-6">
                {[
                  'Event planning resources',
                  'Marketing support',
                  'Speaker connections',
                  'Swag and materials',
                  'Sponsorship opportunities',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <Globe className="w-5 h-5 text-orange-400 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
              <button className="btn-primary">
                Apply to Host
              </button>
            </div>
            <div className="glass-card p-8 text-center">
              <div className="w-24 h-24 rounded-full bg-gradient-to-br from-orange-500 to-amber-500 flex items-center justify-center mx-auto mb-4">
                <MapPin className="w-12 h-12 text-white" />
              </div>
              <h3 className="text-xl font-semibold text-white mb-2">Global Community</h3>
              <p className="text-gray-400">
                X3 STAR events happening in 30+ countries
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Newsletter */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Calendar className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Never Miss an Event
          </h2>
          <p className="text-gray-400 mb-8">
            Subscribe to get notified about upcoming events in your area.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 max-w-md mx-auto">
            <input
              type="email"
              placeholder="Enter your email"
              className="flex-1 px-4 py-3 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a] text-white placeholder-gray-400 focus:outline-none focus:border-orange-500"
            />
            <button className="btn-primary whitespace-nowrap">
              Subscribe
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
