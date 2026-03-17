export type CheckRunInfo = {
  name: string;
  conclusion: string | null;
  status: string;
  detailsUrl: string | null;
};

export type ReviewInfo = {
  user: string;
  state: string;
  submittedAt: string | null;
};

export type PrInfo = {
  number: number;
  title: string;
  state: string;
  isDraft: boolean;
  url: string;
  checks: CheckRunInfo[];
  reviews: ReviewInfo[];
  additions: number | null;
  deletions: number | null;
  reviewDecision: string | null;
};
