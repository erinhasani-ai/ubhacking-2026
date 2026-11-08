import '@fontsource-variable/geist-mono';
import '@fontsource/instrument-serif';
import '../app.css';
import { mount } from 'svelte';
import Phone from './Phone.svelte';

mount(Phone, { target: document.getElementById('app')! });
