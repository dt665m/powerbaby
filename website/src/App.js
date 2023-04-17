import { useState, useEffect } from 'react';
import './App.css';

function App() {
    const [scores, setData] = useState(null);

  useEffect(() => {
    const fetchData = async () => {
      const response = await fetch('result.json');
      const json = await response.json();
      console.log(json);
      setData(json);
    };
    fetchData();
  }, []);

  if (!scores) {
    return <div>Loading...</div>;
  }
  //  const old = {
  //   "personal_blue": {
  //     "a": 30,
  //     "b": 10,
  //     "c": 10,
  //     "d": 20,
  //     "e": 30,
  //     "f": 80,
  //     "g": 20,
  //     "h": 90,
  //   },
  //   "personal_pink": {
  //     "armor": 120,
  //     "j": 3,
  //     "k": 4,
  //     "l": 20,
  //     "m": 20,
  //     "n": 10,
  //     "o": 90,
  //     "p": 20,
  //   },
  //   "blue_total": 0,
  //   "pink_total": 20
  // };
  return (
    <div className="App">
      <header className="App-header">
      <Leaderboard
        pinkScores={scores.personal_pink}
        blueScores={scores.personal_blue}
      />
      </header>
    </div>
  );
}


function Form() {
  const [name, setName] = useState("");
  const [color, setColor] = useState("blue");

  const handleSubmit = (event) => {
    event.preventDefault();
    if (!name || !color) {
      alert("Please fill out all required fields");
      return;
    }
    // window.open(`https://example.com?name=${name}&color=${color}`, "_blank");
    alert("Thanks for joining the game!  Please note:  If the game does not load, you may be on a mobile device that is not supported.  In that case, please retry with a PC or Mac browser.  Sorry! Pro Tip, the game can be zoomed in on a mobile device by double tapping the game window.  You can get better control of the angles.  Good luck!");
    window.open(`https://power-baby.com/game.html?name=${name}&color=${color}`, "_blank");
  };

  return (
    <form onSubmit={handleSubmit} className="form-container">
      <h2 className="form-heading">Gender Vote Game</h2>
      <div className="form-field">
        <label htmlFor="name" className="form-label">Name:</label>
        <input
          type="text"
          id="name"
          required
          value={name}
          onChange={(event) => setName(event.target.value)}
          className="form-input"
        />
      </div>
      <div className="form-field">
        <label htmlFor="color" className="form-label">Color:</label>
        <select
          id="color"
          required
          value={color}
          onChange={(event) => setColor(event.target.value)}
          className="form-select"
        >
          <option value="blue">Blue</option>
          <option value="pink">Pink</option>
        </select>
      </div>
      <button type="submit" className="form-submit">Submit</button>
    </form>
  );
}

const YoutubeEmbed = ({ embedId }) => (
  <div className="video-responsive">
    <iframe
      // width="853"
      // height="480"
      src={`https://www.youtube.com/embed/${embedId}`}
      frameBorder="0"
      allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
      allowFullScreen
      title="Embedded youtube"
    />
  </div>
);

const Leaderboard = ({ pinkScores, blueScores }) => {
  const maxPinkScore = Math.max(...Object.values(pinkScores));
  const maxBlueScore = Math.max(...Object.values(blueScores));
  const maxScore = Math.max(maxPinkScore, maxBlueScore);

  const pinkRows = Object.keys(pinkScores)
    .sort((a, b) => pinkScores[b] - pinkScores[a])
    .map((player) => (
      <div key={player} className="row">
        <div className="player">{player}</div>
        <div className="score">{pinkScores[player]}</div>
        <div className="bar" style={{ width: `${(pinkScores[player] / maxScore) * 100}%` }}></div>
      </div>
    ));

  const blueRows = Object.keys(blueScores)
    .sort((a, b) => blueScores[b] - blueScores[a])
    .map((player) => (
      <div key={player} className="row">
        <div className="player">{player}</div>
        <div className="score">{blueScores[player]}</div>
        <div className="bar" style={{ width: `${(blueScores[player] / maxScore) * 100}%` }}></div>
      </div>
    ));

  const pinkTotal = Object.values(pinkScores).reduce((total, score) => total + score, 0);
  const blueTotal = Object.values(blueScores).reduce((total, score) => total + score, 0);

  return (
    <div className="leaderboard">
      <div className="pink-scores">
        <h2>Pink Team({pinkTotal})</h2>
        {pinkRows}
      </div>
      <div className="blue-scores">
        <h2>Blue Team({blueTotal})</h2>
        {blueRows}
      </div>
    </div>
  );
};

export default App;

