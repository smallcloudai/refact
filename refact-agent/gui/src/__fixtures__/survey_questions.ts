import { SurveyQuestions } from "../services/smallcloud";

export const QUESTIONS_STUB: SurveyQuestions = [
  {
    question: "Where did you first hear about Refact.ai?",
    name: "first_hear_about_refact",
    type: "radio",
    options: [
      {
        title: "Search",
        value: "search",
      },
      {
        title: "IDE Plugin Marketplace",
        value: "ide_plugin_marketplace",
      },
      {
        title: "Social Media",
        value: "social_media",
      },
      {
        title: "Recommendation",
        value: "recommendation",
      },
      {
        title: "Email",
        value: "email",
      },
      {
        title: "Conference",
        value: "conference",
      },
      {
        title: "Article",
        value: "article",
      },
      {
        title: "Aggregator",
        value: "aggregator",
      },
      {
        title: "Stack Overflow",
        value: "stack_overflow",
      },
      {
        title: "Reddit",
        value: "reddit",
      },
      {
        title: "Advertisement",
        value: "advertisement",
      },
      {
        title: "Other",
        value: "other",
      },
    ],
  },
];
