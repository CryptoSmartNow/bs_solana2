## How I Set Up the Environment

Before any tests run, I initialize a local environment that mimics a real-world protocol deployment. This involves setting up several key actors and assets.

### The Actors in My Tests

- **Admin**: This is the default provider wallet. I use this account as the main protocol authority to initialize the system.
- **User**: I generate a random keypair for every test run to represent a new protocol participant.

### Preparing the Assets

To test the protocol's multi-token support, I create several mints:

- **Stable Coin Mint**: I set up a custom SPL token mint to act as our primary stablecoin (like USDC).
- **CS Token Mint**: I initialize this second mint for protocol-specific utility testing.
- **Native SOL**: I use this for registration fees and testing the native savings logic.

### Deriving the PDAs

Solana's state management relies on Program Derived Addresses. I derive several specific PDAs that my tests will interact with:

- **Global State**: This is the singleton account where I store protocol-wide configurations.
- **User Vault**: I derive this profile account directly from the user's public key.
- **Savings Accounts**: I create specific PDAs for different goals, such as "My_SOL_Saving" and "My_Token_Saving," linked back to the user vault.

## My Pre-Test Workflow (The Before Hook)

I use a `before` hook to ensure the environment is ready for testing. During this phase:

1. I airdrop 10 SOL to the user wallet so they have enough funds for fees and savings.
2. I initialize the token mints.
3. I create an Associated Token Account (ATA) for the user and mint 1,000 tokens into it.
4. I also create a vault token account owned by the User Vault PDA, which is where the protocol will securely store the user's saved SPL tokens.

## Walking Through the Test Cases

### 1. Starting the Protocol

First, I call the `initialize` instruction. I have the admin set the protocol state, specifying the admin public key and the supported mints. To verify this worked, I fetch the `globalState` account and confirm the data on-chain matches what I sent.

### 2. Onboarding a User

Next, I simulate a user joining Bitsave. The user calls `joinBitsave`, pays their registration fee, and the program initializes their `UserVault` PDA. I then check the vault account to ensure the owner is correct and the points balance starts at zero.

### 3. Creating a Native SOL Saving

I then test the native savings path. I have the user create a plan with 1 SOL and a maturity time set just 10 seconds into the future. I verify the `saving` account reflects the correct amount and is marked as valid.

### 4. Testing SPL Token Savings

For token-based savings, I have the user deposit 100 SPL tokens. I watch as the program transfers these tokens from the user's ATA to the vault's ATA. My verification step ensures the `saving` account correctly tracks the stablecoin mint I used.

### 5. Adding More Funds

To test the increment logic, I have the user add an extra 0.5 SOL to their existing plan. I then fetch the updated `saving` account and assert that the new total is exactly equal to the original deposit plus the increment.

### 6. Simulating a Premature Withdrawal

Finally, I test the penalty logic. Since I set the maturity to only 10 seconds, I trigger a withdrawal immediately.

- I expect the program to apply a 10% penalty.
- I verify that the remaining funds are sent back to the user.
- I confirm that the `saving` account is closed to reclaim rent, meaning any further attempts to fetch that account should fail.

## Technical Details to Keep in Mind

### Who Owns the Funds?

Throughout my tests, I am careful to verify that funds are held in accounts controlled by PDAs, not by the program directly. For SPL tokens, this means using ATAs where the owner is the `UserVault` PDA.

### Signing Transactions

I ensure that the `user` signs most instructions to authorize fund movements. However, for the `initialize` step, only the `admin` signature is valid.

### Handling Time

Because I use short maturity durations (10 seconds), I can effectively test both the locked-in state and the early-withdrawal penalty logic without waiting for real-world time to pass.
